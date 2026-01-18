//! Aster Agent 命令模块
//!
//! 提供基于 Aster 框架的 Tauri 命令
//! 这是新的对话系统实现，与 native_agent_cmd.rs 并行存在

use crate::agent::aster_state::{ProviderConfig, SessionConfigBuilder};
use crate::agent::event_converter::convert_agent_event;
use crate::agent::{
    AsterAgentState, AsterAgentWrapper, SessionDetail, SessionInfo, TauriAgentEvent,
};
use aster::conversation::message::Message;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

/// Aster Agent 状态信息
#[derive(Debug, Serialize)]
pub struct AsterAgentStatus {
    pub initialized: bool,
    pub provider_configured: bool,
    pub provider_name: Option<String>,
    pub model_name: Option<String>,
}

/// Provider 配置请求
#[derive(Debug, Deserialize)]
pub struct ConfigureProviderRequest {
    pub provider_name: String,
    pub model_name: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// 初始化 Aster Agent
#[tauri::command]
pub async fn aster_agent_init(
    state: State<'_, AsterAgentState>,
) -> Result<AsterAgentStatus, String> {
    tracing::info!("[AsterAgent] 初始化 Agent");

    state.init_agent().await?;

    let provider_config = state.get_provider_config().await;

    tracing::info!("[AsterAgent] Agent 初始化成功");

    Ok(AsterAgentStatus {
        initialized: true,
        provider_configured: provider_config.is_some(),
        provider_name: provider_config.as_ref().map(|c| c.provider_name.clone()),
        model_name: provider_config.as_ref().map(|c| c.model_name.clone()),
    })
}

/// 配置 Aster Agent 的 Provider
#[tauri::command]
pub async fn aster_agent_configure_provider(
    state: State<'_, AsterAgentState>,
    request: ConfigureProviderRequest,
    session_id: String,
) -> Result<AsterAgentStatus, String> {
    tracing::info!(
        "[AsterAgent] 配置 Provider: {} / {}",
        request.provider_name,
        request.model_name
    );

    let config = ProviderConfig {
        provider_name: request.provider_name,
        model_name: request.model_name,
        api_key: request.api_key,
        base_url: request.base_url,
    };

    state
        .configure_provider(config.clone(), &session_id)
        .await?;

    Ok(AsterAgentStatus {
        initialized: true,
        provider_configured: true,
        provider_name: Some(config.provider_name),
        model_name: Some(config.model_name),
    })
}

/// 获取 Aster Agent 状态
#[tauri::command]
pub async fn aster_agent_status(
    state: State<'_, AsterAgentState>,
) -> Result<AsterAgentStatus, String> {
    let provider_config = state.get_provider_config().await;
    Ok(AsterAgentStatus {
        initialized: state.is_initialized().await,
        provider_configured: provider_config.is_some(),
        provider_name: provider_config.as_ref().map(|c| c.provider_name.clone()),
        model_name: provider_config.as_ref().map(|c| c.model_name.clone()),
    })
}

/// 发送消息请求参数
#[derive(Debug, Deserialize)]
pub struct AsterChatRequest {
    pub message: String,
    pub session_id: String,
    pub event_name: String,
    #[serde(default)]
    pub images: Option<Vec<ImageInput>>,
    /// Provider 配置（可选，如果未配置则使用当前配置）
    #[serde(default)]
    pub provider_config: Option<ConfigureProviderRequest>,
}

/// 图片输入
#[derive(Debug, Deserialize)]
pub struct ImageInput {
    pub data: String,
    pub media_type: String,
}

/// 发送消息并获取流式响应
#[tauri::command]
pub async fn aster_agent_chat_stream(
    app: AppHandle,
    state: State<'_, AsterAgentState>,
    request: AsterChatRequest,
) -> Result<(), String> {
    tracing::info!(
        "[AsterAgent] 发送流式消息: session={}, event={}",
        request.session_id,
        request.event_name
    );

    // 确保 Agent 已初始化
    if !state.is_initialized().await {
        state.init_agent().await?;
    }

    // 如果提供了 Provider 配置，则配置 Provider
    if let Some(provider_config) = &request.provider_config {
        let config = ProviderConfig {
            provider_name: provider_config.provider_name.clone(),
            model_name: provider_config.model_name.clone(),
            api_key: provider_config.api_key.clone(),
            base_url: provider_config.base_url.clone(),
        };
        state
            .configure_provider(config, &request.session_id)
            .await?;
    }

    // 检查 Provider 是否已配置
    if !state.is_provider_configured().await {
        return Err("Provider 未配置，请先调用 aster_agent_configure_provider".to_string());
    }

    // 创建取消令牌
    let cancel_token = state.create_cancel_token(&request.session_id).await;

    // 创建用户消息
    let user_message = Message::user().with_text(&request.message);

    // 创建会话配置
    let session_config = SessionConfigBuilder::new(&request.session_id).build();

    // 获取 Agent Arc 并保持 guard 在整个流处理期间存活
    let agent_arc = state.get_agent_arc();
    let guard = agent_arc.read().await;
    let agent = guard.as_ref().ok_or("Agent not initialized")?;

    // 获取事件流
    let stream_result = agent
        .reply(user_message, session_config, Some(cancel_token.clone()))
        .await;

    match stream_result {
        Ok(mut stream) => {
            // 处理事件流
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(agent_event) => {
                        // 转换 Aster 事件为 Tauri 事件
                        let tauri_events = convert_agent_event(agent_event);

                        // 发送每个事件到前端
                        for tauri_event in tauri_events {
                            if let Err(e) = app.emit(&request.event_name, &tauri_event) {
                                tracing::error!("[AsterAgent] 发送事件失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        // 发送错误事件
                        let error_event = TauriAgentEvent::Error {
                            message: format!("Stream error: {}", e),
                        };
                        if let Err(emit_err) = app.emit(&request.event_name, &error_event) {
                            tracing::error!("[AsterAgent] 发送错误事件失败: {}", emit_err);
                        }
                    }
                }
            }

            // 发送完成事件
            let done_event = TauriAgentEvent::FinalDone { usage: None };
            if let Err(e) = app.emit(&request.event_name, &done_event) {
                tracing::error!("[AsterAgent] 发送完成事件失败: {}", e);
            }
        }
        Err(e) => {
            // 发送错误事件
            let error_event = TauriAgentEvent::Error {
                message: format!("Agent error: {}", e),
            };
            if let Err(emit_err) = app.emit(&request.event_name, &error_event) {
                tracing::error!("[AsterAgent] 发送错误事件失败: {}", emit_err);
            }
            return Err(format!("Agent error: {}", e));
        }
    }

    // guard 会在函数结束时自动释放（stream_result 先释放）

    // 清理取消令牌
    state.remove_cancel_token(&request.session_id).await;

    Ok(())
}

/// 停止当前会话
#[tauri::command]
pub async fn aster_agent_stop(
    state: State<'_, AsterAgentState>,
    session_id: String,
) -> Result<bool, String> {
    tracing::info!("[AsterAgent] 停止会话: {}", session_id);
    Ok(state.cancel_session(&session_id).await)
}

/// 创建新会话
#[tauri::command]
pub async fn aster_session_create(
    working_dir: Option<String>,
    name: Option<String>,
) -> Result<String, String> {
    tracing::info!("[AsterAgent] 创建会话: name={:?}", name);
    let dir = working_dir.map(PathBuf::from);
    AsterAgentWrapper::create_session(dir, name).await
}

/// 列出所有会话
#[tauri::command]
pub async fn aster_session_list() -> Result<Vec<SessionInfo>, String> {
    tracing::info!("[AsterAgent] 列出会话");
    AsterAgentWrapper::list_sessions().await
}

/// 获取会话详情
#[tauri::command]
pub async fn aster_session_get(session_id: String) -> Result<SessionDetail, String> {
    tracing::info!("[AsterAgent] 获取会话: {}", session_id);
    AsterAgentWrapper::get_session(&session_id).await
}

/// 确认权限请求
#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    pub request_id: String,
    pub confirmed: bool,
    pub response: Option<String>,
}

/// 确认权限请求（用于工具调用确认等）
#[tauri::command]
pub async fn aster_agent_confirm(
    _state: State<'_, AsterAgentState>,
    request: ConfirmRequest,
) -> Result<(), String> {
    tracing::info!(
        "[AsterAgent] 确认请求: id={}, confirmed={}",
        request.request_id,
        request.confirmed
    );

    // TODO: 实现权限确认逻辑
    // 这需要 Aster 框架支持 confirmation_tx 通道
    // 目前先返回成功

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aster_chat_request_deserialize() {
        let json = r#"{
            "message": "Hello",
            "session_id": "test-session",
            "event_name": "agent_stream"
        }"#;

        let request: AsterChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message, "Hello");
        assert_eq!(request.session_id, "test-session");
        assert_eq!(request.event_name, "agent_stream");
    }
}
