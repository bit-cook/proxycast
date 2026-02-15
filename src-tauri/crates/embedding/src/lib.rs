//! 向量嵌入服务
//!
//! 提供文本向量化功能，用于语义搜索

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// OpenAI Embedding API 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// 输入文本
    pub input: String,
    /// 模型名称（默认 text-embedding-3-small）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// OpenAI Embedding API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

/// 向量数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    /// 向量数组（768 维 for text-embedding-3-small）
    pub embedding: Vec<f32>,
    /// 索引
    pub index: usize,
}

/// API 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: ApiError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

/// 获取文本向量嵌入
///
/// # 参数
///
/// * `text` - 要向量化的文本
/// * `api_key` - OpenAI API 密钥
/// * `model` - 模型名称（可选，默认 text-embedding-3-small）
///
/// # 返回
///
/// 成功时返回向量数组（768 维 f32），失败时返回错误信息
///
/// # 示例
///
/// ```ignore
/// use proxycast_embedding::get_embedding;
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
///     let api_key = "sk-...";
///     let text = "我喜欢喝咖啡";
///
///     match get_embedding(text, api_key, None).await {
///         Ok(embedding) => println!("向量维度: {}", embedding.len()),
///         Err(e) => eprintln!("错误: {}", e),
///     }
/// });
/// ```
pub async fn get_embedding(
    text: &str,
    api_key: &str,
    model: Option<&str>,
) -> Result<Vec<f32>, String> {
    tracing::debug!(
        "[嵌入服务] 请求嵌入: text_len={}, model={:?}",
        text.len(),
        model
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let model = model.unwrap_or("text-embedding-3-small");

    let req = EmbeddingRequest {
        input: text.to_string(),
        model: Some(model.to_string()),
    };

    let url = "https://api.openai.com/v1/embeddings";

    tracing::debug!("[嵌入服务] 发送请求到: {}", url);

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    tracing::debug!("[嵌入服务] 响应状态: {}", resp.status());

    if resp.status() != 200 {
        let status = resp.status();
        let error_text = resp
            .text()
            .await
            .unwrap_or_else(|e| format!("读取错误响应失败: {e}"));

        tracing::error!("[嵌入服务] API 错误: {} - {}", status, error_text);

        return Err(format!("API 错误: {status} - {error_text}"));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("读取响应体失败: {e}"))?;

    tracing::debug!("[嵌入服务] 响应体长度: {} bytes", body.len());

    let response: EmbeddingResponse =
        serde_json::from_str(&body).map_err(|e| format!("JSON 解析失败: {e}"))?;

    if response.data.is_empty() {
        return Err("API 返回数据为空".to_string());
    }

    let embedding = &response.data[0].embedding;

    tracing::debug!("[嵌入服务] 向量维度: {}", embedding.len());

    Ok(embedding.clone())
}

/// 批量获取向量嵌入
///
/// # 参数
///
/// * `texts` - 文本列表
/// * `api_key` - OpenAI API 密钥
/// * `model` - 模型名称（可选）
///
/// # 返回
///
/// 成功时返回向量列表
pub async fn get_embeddings_batch(
    texts: &[String],
    api_key: &str,
    model: Option<&str>,
) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    tracing::info!("[嵌入服务] 批量嵌入: count={}", texts.len());

    // 并发请求，限制并发数为 10
    let mut tasks = Vec::new();
    for chunk in texts.chunks(10) {
        for text in chunk {
            let text = text.clone();
            let api_key = api_key.to_string();
            let model = model.map(|s| s.to_string());

            let task =
                tokio::spawn(async move { get_embedding(&text, &api_key, model.as_deref()).await });

            tasks.push(task);
        }
    }

    let mut results = Vec::with_capacity(texts.len());
    let mut errors = Vec::new();

    for task in tasks {
        match task.await.map_err(|e| format!("任务失败: {e}"))? {
            Ok(embedding) => results.push(embedding),
            Err(e) => {
                tracing::warn!("[嵌入服务] 批量中单个失败: {}", e);
                errors.push(e);
                results.push(vec![]); // 占位
            }
        }
    }

    if !errors.is_empty() {
        tracing::warn!("[嵌入服务] 批量完成，但有 {} 个失败", errors.len());
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_embedding_mock() {
        // 这个测试需要真实的 API key，在 CI 中跳过
        let api_key = std::env::var("OPENAI_API_KEY");
        if api_key.is_err() {
            println!("跳过测试：未设置 OPENAI_API_KEY");
            return;
        }

        let api_key = api_key.unwrap();
        let text = "测试文本";

        match get_embedding(text, &api_key, None).await {
            Ok(embedding) => {
                assert_eq!(embedding.len(), 1536); // text-embedding-3-small 是 1536 维
                println!("向量前 5 维: {:?}", &embedding[..5]);
            }
            Err(e) => {
                eprintln!("测试失败: {e}");
            }
        }
    }

    #[test]
    fn test_embedding_request_serialization() {
        let req = EmbeddingRequest {
            input: "测试".to_string(),
            model: Some("text-embedding-3-small".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""input":"测试""#));
        assert!(json.contains(r#""model":"text-embedding-3-small""#));
    }
}
