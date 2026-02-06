//! Aster Agent 包装器
//!
//! 提供简化的接口来使用 Aster Agent
//! 处理消息发送、事件流转换和会话管理

use crate::agent::aster_state::{AsterAgentState, SessionConfigBuilder};
use crate::database::dao::agent::AgentDao;
use crate::database::DbConnection;
use aster::conversation::message::Message;
use chrono::Utc;
use futures::StreamExt;
use tauri::{AppHandle, Emitter};

/// Aster Agent 包装器
///
/// 提供与 Tauri 集成的简化接口
pub struct AsterAgentWrapper;

impl AsterAgentWrapper {
    /// 发送消息并获取流式响应
    ///
    /// # Arguments
    /// * `state` - Aster Agent 状态
    /// * `db` - 数据库连接
    /// * `app` - Tauri AppHandle，用于发送事件
    /// * `message` - 用户消息文本
    /// * `session_id` - 会话 ID
    /// * `event_name` - 前端监听的事件名称
    ///
    /// # Returns
    /// 成功时返回 Ok(())，失败时返回错误信息
    pub async fn send_message(
        state: &AsterAgentState,
        db: &DbConnection,
        app: &AppHandle,
        message: String,
        session_id: String,
        event_name: String,
    ) -> Result<(), String> {
        // 1. 初始化检查（使用带数据库的版本）
        if !state.is_initialized().await {
            state.init_agent_with_db(db).await?;
        }

        // 2. 创建取消令牌
        let cancel_token = state.create_cancel_token(&session_id).await;

        // 3. 构建消息和配置
        let user_message = Message::user().with_text(&message);
        let session_config = SessionConfigBuilder::new(&session_id).build();

        // 4. 获取 Agent 引用（关键步骤）
        let agent_arc = state.get_agent_arc();
        let guard = agent_arc.read().await;
        let agent = guard.as_ref().ok_or("Agent not initialized")?;

        // 5. 调用 Agent::reply
        let stream_result = agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
            .await;

        // 6. 处理流式响应
        match stream_result {
            Ok(mut stream) => {
                while let Some(event_result) = stream.next().await {
                    match event_result {
                        Ok(agent_event) => {
                            // 转换并发送事件到前端
                            let tauri_events =
                                crate::agent::event_converter::convert_agent_event(agent_event);
                            for tauri_event in tauri_events {
                                if let Err(e) = app.emit(&event_name, &tauri_event) {
                                    tracing::error!("[AsterAgentWrapper] 发送事件失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            // 发送错误事件
                            let error_event =
                                crate::agent::event_converter::TauriAgentEvent::Error {
                                    message: format!("Stream error: {e}"),
                                };
                            let _ = app.emit(&event_name, &error_event);
                        }
                    }
                }

                // 发送完成事件
                let done_event =
                    crate::agent::event_converter::TauriAgentEvent::FinalDone { usage: None };
                let _ = app.emit(&event_name, &done_event);
            }
            Err(e) => {
                // 发送错误事件并返回错误
                let error_event = crate::agent::event_converter::TauriAgentEvent::Error {
                    message: format!("Agent error: {e}"),
                };
                let _ = app.emit(&event_name, &error_event);
                return Err(format!("Agent error: {e}"));
            }
        }

        // guard 在作用域结束时自动释放

        // 7. 清理取消令牌
        state.remove_cancel_token(&session_id).await;

        Ok(())
    }

    /// 停止当前会话
    pub async fn stop_session(state: &AsterAgentState, session_id: &str) -> bool {
        state.cancel_session(session_id).await
    }

    /// 创建新会话 - 使用 ProxyCast 数据库
    pub fn create_session_sync(db: &DbConnection, name: Option<String>) -> Result<String, String> {
        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
        let session_name = name.unwrap_or_else(|| "新对话".to_string());
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let session = crate::agent::types::AgentSession {
            id: session_id.clone(),
            model: "agent:default".to_string(),
            messages: Vec::new(),
            system_prompt: None,
            title: Some(session_name),
            created_at: now.clone(),
            updated_at: now,
        };

        AgentDao::create_session(&conn, &session).map_err(|e| format!("创建会话失败: {e}"))?;

        Ok(session_id)
    }

    /// 列出所有会话 - 使用 ProxyCast 数据库
    pub fn list_sessions_sync(db: &DbConnection) -> Result<Vec<SessionInfo>, String> {
        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

        let sessions =
            AgentDao::list_sessions(&conn).map_err(|e| format!("获取会话列表失败: {e}"))?;

        Ok(sessions
            .into_iter()
            .map(|s| {
                let messages_count = AgentDao::get_message_count(&conn, &s.id).unwrap_or(0);
                SessionInfo {
                    id: s.id,
                    name: s.title.unwrap_or_else(|| "未命名".to_string()),
                    created_at: chrono::DateTime::parse_from_rfc3339(&s.created_at)
                        .map(|dt| dt.timestamp())
                        .unwrap_or(0),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&s.updated_at)
                        .map(|dt| dt.timestamp())
                        .unwrap_or(0),
                    messages_count,
                }
            })
            .collect())
    }

    /// 获取会话详情 - 使用 ProxyCast 数据库
    pub fn get_session_sync(db: &DbConnection, session_id: &str) -> Result<SessionDetail, String> {
        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

        let session = AgentDao::get_session(&conn, session_id)
            .map_err(|e| format!("获取会话失败: {e}"))?
            .ok_or_else(|| format!("会话不存在: {session_id}"))?;

        let messages =
            AgentDao::get_messages(&conn, session_id).map_err(|e| format!("获取消息失败: {e}"))?;

        Ok(SessionDetail {
            id: session.id,
            name: session.title.unwrap_or_else(|| "未命名".to_string()),
            created_at: chrono::DateTime::parse_from_rfc3339(&session.created_at)
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
            updated_at: chrono::DateTime::parse_from_rfc3339(&session.updated_at)
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
            messages: messages
                .into_iter()
                .map(|m| convert_agent_message(&m))
                .collect(),
        })
    }
}

/// 会话信息（简化版）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub messages_count: usize,
}

/// 会话详情（包含消息）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionDetail {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub messages: Vec<crate::agent::event_converter::TauriMessage>,
}

/// 将 AgentMessage 转换为 TauriMessage
fn convert_agent_message(
    msg: &crate::agent::types::AgentMessage,
) -> crate::agent::event_converter::TauriMessage {
    use crate::agent::event_converter::{TauriMessage, TauriMessageContent};
    use crate::agent::types::MessageContent;

    let content = match &msg.content {
        MessageContent::Text(text) => vec![TauriMessageContent::Text { text: text.clone() }],
        MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| {
                if let crate::agent::types::ContentPart::Text { text } = p {
                    Some(TauriMessageContent::Text { text: text.clone() })
                } else {
                    None
                }
            })
            .collect(),
    };

    // 解析时间戳
    let timestamp = chrono::DateTime::parse_from_rfc3339(&msg.timestamp)
        .map(|dt| dt.timestamp())
        .unwrap_or(0);

    TauriMessage {
        id: None,
        role: msg.role.clone(),
        content,
        timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_config_builder() {
        let config = SessionConfigBuilder::new("test-session").build();
        assert_eq!(config.id, "test-session");
    }
}
