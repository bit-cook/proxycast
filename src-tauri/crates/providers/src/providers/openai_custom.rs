//! OpenAI Custom Provider (自定义 OpenAI 兼容 API)
use proxycast_core::models::openai::ChatCompletionRequest;
use reqwest::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAICustomConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub enabled: bool,
}

pub struct OpenAICustomProvider {
    pub config: OpenAICustomConfig,
    pub client: Client,
}

/// 创建配置好的 HTTP 客户端
fn create_http_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(600)) // 10 分钟总超时
        .tcp_keepalive(Duration::from_secs(60))
        .gzip(true) // 自动解压 gzip 响应
        .brotli(true) // 自动解压 brotli 响应
        .deflate(true) // 自动解压 deflate 响应
        .build()
        .unwrap_or_else(|_| Client::new())
}

impl Default for OpenAICustomProvider {
    fn default() -> Self {
        Self {
            config: OpenAICustomConfig::default(),
            client: create_http_client(),
        }
    }
}

impl OpenAICustomProvider {
    fn maybe_log_protocol_mismatch_hint(url: &str, status: StatusCode) {
        if (status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN)
            && url.contains("/api/anthropic")
        {
            eprintln!(
                "[OPENAI_CUSTOM] 提示: URL '{url}' 返回 {status}，疑似协议不匹配。若上游是 Anthropic 兼容网关，请改用 /v1/messages + x-api-key。"
            );
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// 使用 API key 和 base_url 创建 Provider
    pub fn with_config(api_key: String, base_url: Option<String>) -> Self {
        Self {
            config: OpenAICustomConfig {
                api_key: Some(api_key),
                base_url,
                enabled: true,
            },
            client: create_http_client(),
        }
    }

    pub fn get_base_url(&self) -> String {
        self.config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com".to_string())
    }

    pub fn is_configured(&self) -> bool {
        self.config.api_key.is_some() && self.config.enabled
    }

    /// 构建完整的 API URL
    /// 智能处理用户输入的 base_url，支持多种 API 版本格式
    ///
    /// 支持的格式：
    /// - `https://api.openai.com` -> `https://api.openai.com/v1/chat/completions`
    /// - `https://api.openai.com/v1` -> `https://api.openai.com/v1/chat/completions`
    /// - `https://open.bigmodel.cn/api/paas/v4` -> `https://open.bigmodel.cn/api/paas/v4/chat/completions`
    /// - `https://api.deepseek.com/v1` -> `https://api.deepseek.com/v1/chat/completions`
    fn build_url(&self, endpoint: &str) -> String {
        let base = self.get_base_url();
        let base = base.trim_end_matches('/');

        // 检查是否已经包含版本号路径（/v1, /v2, /v3, /v4 等）
        // 使用正则匹配 /v 后跟数字的模式
        let has_version = base
            .rsplit('/')
            .next()
            .map(|last_segment| {
                last_segment.starts_with('v')
                    && last_segment.len() >= 2
                    && last_segment[1..].chars().all(|c| c.is_ascii_digit())
            })
            .unwrap_or(false);

        if has_version {
            // 已有版本号，直接拼接 endpoint
            format!("{base}/{endpoint}")
        } else {
            // 没有版本号，添加 /v1
            format!("{base}/v1/{endpoint}")
        }
    }

    fn build_url_fallback_without_v1(&self, endpoint: &str) -> Option<String> {
        let url = self.build_url(endpoint);
        if url.contains("/v1/") {
            Some(url.replacen("/v1/", "/", 1))
        } else {
            None
        }
    }

    fn build_url_from_base(base_url: &str, endpoint: &str) -> String {
        let base = base_url.trim_end_matches('/');

        let has_version = base
            .rsplit('/')
            .next()
            .map(|last_segment| {
                last_segment.starts_with('v')
                    && last_segment.len() >= 2
                    && last_segment[1..].chars().all(|c| c.is_ascii_digit())
            })
            .unwrap_or(false);

        if has_version {
            format!("{base}/{endpoint}")
        } else {
            format!("{base}/v1/{endpoint}")
        }
    }

    fn base_url_parent(&self) -> Option<String> {
        let base = self.get_base_url();
        let base = base.trim();

        let mut url = Url::parse(base)
            .or_else(|_| Url::parse(&format!("http://{base}")))
            .ok()?;

        let path = url.path().trim_end_matches('/');
        if path.is_empty() || path == "/" {
            return None;
        }

        let mut segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            return None;
        }
        segments.pop();

        let new_path = if segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", segments.join("/"))
        };

        url.set_path(&new_path);
        url.set_query(None);
        url.set_fragment(None);

        Some(url.to_string().trim_end_matches('/').to_string())
    }

    fn build_urls_with_fallbacks(&self, endpoint: &str) -> Vec<String> {
        let mut urls: Vec<String> = Vec::new();

        let primary = self.build_url(endpoint);
        urls.push(primary.clone());

        if let Some(no_v1) = self.build_url_fallback_without_v1(endpoint) {
            if no_v1 != primary {
                urls.push(no_v1);
            }
        }

        if let Some(parent_base) = self.base_url_parent() {
            let u = Self::build_url_from_base(&parent_base, endpoint);
            if !urls.iter().any(|x| x == &u) {
                urls.push(u.clone());
            }

            if u.contains("/v1/") {
                let u2 = u.replacen("/v1/", "/", 1);
                if !urls.iter().any(|x| x == &u2) {
                    urls.push(u2);
                }
            }
        }

        urls
    }

    /// 调用 OpenAI API（使用类型化请求）
    pub async fn call_api(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or("OpenAI API key not configured")?;

        let urls = self.build_urls_with_fallbacks("chat/completions");
        let mut last_resp: Option<reqwest::Response> = None;

        eprintln!(
            "[OPENAI_CUSTOM] call_api testing with model: {}",
            request.model
        );

        for url in &urls {
            eprintln!("[OPENAI_CUSTOM] call_api trying URL: {url}");
            let resp = self
                .client
                .post(url)
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .json(request)
                .send()
                .await?;

            Self::maybe_log_protocol_mismatch_hint(url, resp.status());

            if resp.status() != StatusCode::NOT_FOUND {
                return Ok(resp);
            }
            last_resp = Some(resp);
        }

        Ok(last_resp.ok_or("Request failed")?)
    }

    pub async fn chat_completions(
        &self,
        request: &serde_json::Value,
    ) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or("OpenAI API key not configured")?;

        let url = self.build_url("chat/completions");

        eprintln!("[OPENAI_CUSTOM] chat_completions URL: {url}");
        eprintln!(
            "[OPENAI_CUSTOM] chat_completions base_url: {}",
            self.get_base_url()
        );

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        Self::maybe_log_protocol_mismatch_hint(&url, resp.status());

        if resp.status() == StatusCode::NOT_FOUND {
            if let Some(fallback_url) = self.build_url_fallback_without_v1("chat/completions") {
                if fallback_url != url {
                    let resp2 = self
                        .client
                        .post(&fallback_url)
                        .header("Authorization", format!("Bearer {api_key}"))
                        .header("Content-Type", "application/json")
                        .json(request)
                        .send()
                        .await?;
                    Self::maybe_log_protocol_mismatch_hint(&fallback_url, resp2.status());
                    return Ok(resp2);
                }
            }
        }

        Ok(resp)
    }

    pub async fn list_models(&self) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or("OpenAI API key not configured")?;

        let urls = self.build_urls_with_fallbacks("models");
        let mut tried_urls: Vec<String> = Vec::new();
        let mut resp: Option<reqwest::Response> = None;

        for url in urls {
            eprintln!("[OPENAI_CUSTOM] list_models URL: {url}");
            tried_urls.push(url.clone());
            let r = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {api_key}"))
                .send()
                .await?;
            Self::maybe_log_protocol_mismatch_hint(&url, r.status());
            if r.status() != StatusCode::NOT_FOUND {
                resp = Some(r);
                break;
            }
            resp = Some(r);
        }

        let resp = resp.ok_or("Request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            eprintln!("[OPENAI_CUSTOM] list_models 失败: {status} - {body}");
            return Err(format!(
                "Failed to list models: {status} - {body} (tried: {})",
                tried_urls.join(", ")
            )
            .into());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data)
    }
}

// ============================================================================
// StreamingProvider Trait 实现
// ============================================================================

use crate::providers::ProviderError;
use crate::streaming::traits::{
    reqwest_stream_to_stream_response, StreamFormat, StreamResponse, StreamingProvider,
};
use async_trait::async_trait;

#[async_trait]
impl StreamingProvider for OpenAICustomProvider {
    /// 发起流式 API 调用
    ///
    /// 使用 reqwest 的 bytes_stream 返回字节流，支持真正的端到端流式传输。
    /// OpenAI 使用 OpenAI SSE 格式。
    ///
    /// # 需求覆盖
    /// - 需求 1.3: OpenAICustomProvider 流式支持
    async fn call_api_stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<StreamResponse, ProviderError> {
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            ProviderError::ConfigurationError("OpenAI API key not configured".to_string())
        })?;

        // 确保请求启用流式
        let mut stream_request = request.clone();
        stream_request.stream = true;

        let url = self.build_url("chat/completions");

        tracing::info!(
            "[OPENAI_STREAM] 发起流式请求: url={} model={}",
            url,
            request.model
        );

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&stream_request)
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest_error(&e))?;

        let resp = if resp.status() == StatusCode::NOT_FOUND {
            if let Some(fallback_url) = self.build_url_fallback_without_v1("chat/completions") {
                if fallback_url != url {
                    self.client
                        .post(&fallback_url)
                        .header("Authorization", format!("Bearer {api_key}"))
                        .header("Content-Type", "application/json")
                        .header("Accept", "text/event-stream")
                        .json(&stream_request)
                        .send()
                        .await
                        .map_err(|e| ProviderError::from_reqwest_error(&e))?
                } else {
                    resp
                }
            } else {
                resp
            }
        } else {
            resp
        };

        // 检查响应状态
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("[OPENAI_STREAM] 请求失败: {} - {}", status, body);
            return Err(ProviderError::from_http_status(status.as_u16(), &body));
        }

        tracing::info!("[OPENAI_STREAM] 流式响应开始: status={}", status);

        // 将 reqwest 响应转换为 StreamResponse
        Ok(reqwest_stream_to_stream_response(resp))
    }

    fn supports_streaming(&self) -> bool {
        self.is_configured()
    }

    fn provider_name(&self) -> &'static str {
        "OpenAICustomProvider"
    }

    fn stream_format(&self) -> StreamFormat {
        StreamFormat::OpenAiSse
    }
}
