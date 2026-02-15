//! 电商差评回复 Tauri 命令
//!
//! 提供电商差评回复的专门接口,封装 Skill 执行逻辑

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::agent::AsterAgentState;
use crate::commands::skill_exec_cmd::{execute_skill, SkillExecutionResult};
use crate::database::DbConnection;

/// 电商差评回复请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcommerceReviewReplyRequest {
    /// 电商平台
    pub platform: String,
    /// 差评链接
    pub review_url: String,
    /// 回复语气
    pub tone: String,
    /// 回复长度
    pub length: String,
    /// 自定义模板 (可选)
    pub template: Option<String>,
    /// AI 模型
    pub model: Option<String>,
    /// 执行 ID (可选)
    pub execution_id: Option<String>,
}

/// 执行电商差评回复
///
/// 这是一个便捷接口,封装了 execute_skill 的调用
///
/// # Arguments
/// * `app_handle` - Tauri AppHandle
/// * `db` - 数据库连接
/// * `aster_state` - Aster Agent 状态
/// * `request` - 电商差评回复请求
///
/// # Returns
/// * `Ok(SkillExecutionResult)` - 执行结果
/// * `Err(String)` - 错误信息
#[tauri::command]
pub async fn execute_ecommerce_review_reply(
    app_handle: tauri::AppHandle,
    db: State<'_, DbConnection>,
    aster_state: State<'_, AsterAgentState>,
    request: EcommerceReviewReplyRequest,
) -> Result<SkillExecutionResult, String> {
    tracing::info!(
        "[execute_ecommerce_review_reply] 开始执行: platform={}, url={}",
        request.platform,
        request.review_url
    );

    // 构建用户输入
    let user_input = format!(
        "平台: {}\n差评链接: {}\n回复语气: {}\n回复长度: {}{}",
        request.platform,
        request.review_url,
        request.tone,
        request.length,
        request
            .template
            .as_ref()
            .map(|t| format!("\n自定义模板: {t}"))
            .unwrap_or_default()
    );

    // 调用通用的 execute_skill
    execute_skill(
        app_handle,
        db,
        aster_state,
        "ecommerce-review-reply".to_string(),
        user_input,
        Some("anthropic".to_string()), // 优先使用 Anthropic
        request.model,
        request.execution_id,
        None, // session_id
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = EcommerceReviewReplyRequest {
            platform: "taobao".to_string(),
            review_url: "https://example.com/review/123".to_string(),
            tone: "sincere".to_string(),
            length: "medium".to_string(),
            template: None,
            model: Some("claude-sonnet-4-5".to_string()),
            execution_id: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: EcommerceReviewReplyRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.platform, "taobao");
        assert_eq!(deserialized.tone, "sincere");
    }
}
