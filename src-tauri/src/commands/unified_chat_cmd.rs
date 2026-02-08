//! 统一对话命令模块
//!
//! 提供统一的对话 API，支持多种对话模式：
//! - Agent: AI Agent 模式，支持工具调用
//! - General: 通用对话模式，纯文本
//! - Creator: 内容创作模式，支持画布输出
//!
//! ## 设计原则
//! - 单一入口：所有对话场景使用同一套 API
//! - 模式化设计：通过 ChatMode 区分不同场景
//! - Aster 引擎：底层使用 Aster Agent 处理对话
//!
//! ## 参考文档
//! - `docs/prd/chat-architecture-redesign.md`

use crate::agent::aster_state::SessionConfigBuilder;
use crate::agent::{AsterAgentState, TauriAgentEvent};
use crate::database::dao::chat::{ChatDao, ChatMessage, ChatMode, ChatSession};
use crate::database::DbConnection;
use aster::conversation::message::Message;
use futures::StreamExt;
use proxycast_agent::event_converter::convert_agent_event;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

// ============================================================================
// 请求/响应结构
// ============================================================================

/// 创建会话请求
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    /// 对话模式
    pub mode: ChatMode,
    /// 会话标题（可选）
    pub title: Option<String>,
    /// 系统提示词（可选）
    pub system_prompt: Option<String>,
    /// Provider 类型（可选）
    pub provider_type: Option<String>,
    /// 模型名称（可选）
    pub model: Option<String>,
    /// 扩展元数据（可选）
    pub metadata: Option<serde_json::Value>,
}

/// 发送消息请求
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    /// 会话 ID
    pub session_id: String,
    /// 消息内容
    pub message: String,
    /// 事件名称（用于前端监听）
    pub event_name: String,
    /// 图片输入（可选，用于多模态对话）
    /// TODO: 实现图片处理逻辑，将图片转换为 Aster Message 的 ImageContent
    pub images: Option<Vec<ImageInput>>,
}

/// 图片输入
#[derive(Debug, Deserialize)]
pub struct ImageInput {
    /// Base64 编码的图片数据
    pub data: String,
    /// 图片 MIME 类型，如 "image/png", "image/jpeg"
    pub media_type: String,
}

/// 会话信息响应
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub mode: ChatMode,
    pub title: Option<String>,
    pub model: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
}

impl From<ChatSession> for SessionResponse {
    fn from(session: ChatSession) -> Self {
        Self {
            id: session.id,
            mode: session.mode,
            title: session.title,
            model: session.model,
            created_at: session.created_at,
            updated_at: session.updated_at,
            message_count: 0,
        }
    }
}

// ============================================================================
// 会话管理命令
// ============================================================================

/// 创建新会话
///
/// 统一的会话创建入口，支持所有对话模式
#[tauri::command]
pub async fn chat_create_session(
    db: State<'_, DbConnection>,
    agent_state: State<'_, AsterAgentState>,
    request: CreateSessionRequest,
) -> Result<SessionResponse, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let session_id = uuid::Uuid::new_v4().to_string();

    // 创建会话
    let session = ChatSession {
        id: session_id.clone(),
        mode: request.mode,
        title: request.title,
        system_prompt: request.system_prompt.clone(),
        model: request.model.clone(),
        provider_type: request.provider_type.clone(),
        credential_uuid: None,
        metadata: request.metadata,
        created_at: now.clone(),
        updated_at: now,
    };

    // 保存到数据库
    {
        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
        ChatDao::create_session(&conn, &session).map_err(|e| format!("创建会话失败: {e}"))?;
    }

    // 初始化 Aster Agent（如果是 Agent 或 Creator 模式）
    if matches!(request.mode, ChatMode::Agent | ChatMode::Creator) {
        agent_state.init_agent_with_db(&db).await?;

        // 如果指定了 Provider，配置它
        if let (Some(provider_type), Some(model)) = (&request.provider_type, &request.model) {
            agent_state
                .configure_provider_from_pool(&db, provider_type, model, &session_id)
                .await?;
        }
    }

    tracing::info!(
        "[UnifiedChat] 创建会话: id={}, mode={:?}",
        session_id,
        request.mode
    );

    Ok(SessionResponse::from(session))
}

/// 获取会话列表
///
/// 可选按模式过滤
#[tauri::command]
pub async fn chat_list_sessions(
    db: State<'_, DbConnection>,
    mode: Option<ChatMode>,
) -> Result<Vec<SessionResponse>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let sessions =
        ChatDao::list_sessions(&conn, mode).map_err(|e| format!("获取会话列表失败: {e}"))?;

    let mut result: Vec<SessionResponse> = Vec::new();
    for session in sessions {
        let message_count = ChatDao::get_message_count(&conn, &session.id).unwrap_or(0);
        let mut resp = SessionResponse::from(session);
        resp.message_count = message_count;
        result.push(resp);
    }

    Ok(result)
}

/// 获取会话详情
#[tauri::command]
pub async fn chat_get_session(
    db: State<'_, DbConnection>,
    session_id: String,
) -> Result<SessionResponse, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let session = ChatDao::get_session(&conn, &session_id)
        .map_err(|e| format!("获取会话失败: {e}"))?
        .ok_or_else(|| "会话不存在".to_string())?;

    let message_count = ChatDao::get_message_count(&conn, &session_id).unwrap_or(0);
    let mut resp = SessionResponse::from(session);
    resp.message_count = message_count;

    Ok(resp)
}

/// 删除会话
#[tauri::command]
pub async fn chat_delete_session(
    db: State<'_, DbConnection>,
    session_id: String,
) -> Result<bool, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let deleted =
        ChatDao::delete_session(&conn, &session_id).map_err(|e| format!("删除会话失败: {e}"))?;

    if deleted {
        tracing::info!("[UnifiedChat] 删除会话: id={}", session_id);
    }

    Ok(deleted)
}

/// 重命名会话
#[tauri::command]
pub async fn chat_rename_session(
    db: State<'_, DbConnection>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    ChatDao::update_title(&conn, &session_id, &title)
        .map_err(|e| format!("重命名会话失败: {e}"))?;

    tracing::info!(
        "[UnifiedChat] 重命名会话: id={}, title={}",
        session_id,
        title
    );

    Ok(())
}

// ============================================================================
// 消息管理命令
// ============================================================================

/// 获取会话消息列表
#[tauri::command]
pub async fn chat_get_messages(
    db: State<'_, DbConnection>,
    session_id: String,
    limit: Option<i32>,
) -> Result<Vec<ChatMessage>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let messages = ChatDao::get_messages(&conn, &session_id, limit)
        .map_err(|e| format!("获取消息失败: {e}"))?;

    Ok(messages)
}

/// 发送消息并获取流式响应
///
/// 统一的消息发送入口，根据会话模式选择处理方式
#[tauri::command]
pub async fn chat_send_message(
    app: AppHandle,
    db: State<'_, DbConnection>,
    agent_state: State<'_, AsterAgentState>,
    request: SendMessageRequest,
) -> Result<(), String> {
    let image_count = request.images.as_ref().map(|v| v.len()).unwrap_or(0);
    tracing::info!(
        "[UnifiedChat] 发送消息: session={}, event={}, images={}",
        request.session_id,
        request.event_name,
        image_count
    );

    // TODO: 实现图片处理逻辑，将图片转换为 Aster Message 的 ImageContent
    if let Some(images) = &request.images {
        for (i, img) in images.iter().enumerate() {
            tracing::debug!(
                "[UnifiedChat] 图片 {}: media_type={}, data_len={}",
                i,
                img.media_type,
                img.data.len()
            );
        }
        if !images.is_empty() {
            tracing::warn!(
                "[UnifiedChat] 图片输入暂未实现，忽略 {} 张图片",
                images.len()
            );
        }
    }

    // 获取会话信息
    let session = {
        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
        ChatDao::get_session(&conn, &request.session_id)
            .map_err(|e| format!("获取会话失败: {e}"))?
            .ok_or_else(|| "会话不存在".to_string())?
    };

    // 根据模式处理
    match session.mode {
        ChatMode::Agent | ChatMode::Creator => {
            // 使用 Aster Agent 处理
            send_message_with_aster(
                &app,
                &db,
                &agent_state,
                &request.session_id,
                &request.message,
                &request.event_name,
                session.system_prompt.as_deref(),
            )
            .await
        }
        ChatMode::General => {
            // 通用模式：也使用 Aster Agent，但不启用工具
            send_message_with_aster(
                &app,
                &db,
                &agent_state,
                &request.session_id,
                &request.message,
                &request.event_name,
                session.system_prompt.as_deref(),
            )
            .await
        }
    }
}

/// 使用 Aster Agent 发送消息
async fn send_message_with_aster(
    app: &AppHandle,
    db: &DbConnection,
    agent_state: &AsterAgentState,
    session_id: &str,
    message: &str,
    event_name: &str,
    system_prompt: Option<&str>,
) -> Result<(), String> {
    // 确保 Agent 已初始化
    if !agent_state.is_initialized().await {
        agent_state.init_agent_with_db(db).await?;
    }

    // 检查 Provider 是否已配置
    if !agent_state.is_provider_configured().await {
        return Err("Provider 未配置，请先配置凭证".to_string());
    }

    // 创建取消令牌
    let cancel_token = agent_state.create_cancel_token(session_id).await;

    // 构建消息（如果有 system_prompt 且是第一条消息，注入到消息前面）
    let final_message = if let Some(prompt) = system_prompt {
        format!("{prompt}\n\n{message}")
    } else {
        message.to_string()
    };

    let user_message = Message::user().with_text(&final_message);
    let session_config = SessionConfigBuilder::new(session_id).build();

    // 获取 Agent 引用
    let agent_arc = agent_state.get_agent_arc();
    let guard = agent_arc.read().await;
    let agent = guard.as_ref().ok_or("Agent 未初始化")?;

    // 调用 Agent
    let stream_result = agent
        .reply(user_message, session_config, Some(cancel_token.clone()))
        .await;

    match stream_result {
        Ok(mut stream) => {
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(agent_event) => {
                        let tauri_events = convert_agent_event(agent_event);
                        for tauri_event in tauri_events {
                            if let Err(e) = app.emit(event_name, &tauri_event) {
                                tracing::error!("[UnifiedChat] 发送事件失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        let error_event = TauriAgentEvent::Error {
                            message: format!("流错误: {e}"),
                        };
                        let _ = app.emit(event_name, &error_event);
                    }
                }
            }

            // 发送完成事件
            let done_event = TauriAgentEvent::FinalDone { usage: None };
            let _ = app.emit(event_name, &done_event);
        }
        Err(e) => {
            let error_event = TauriAgentEvent::Error {
                message: format!("Agent 错误: {e}"),
            };
            let _ = app.emit(event_name, &error_event);
            return Err(format!("Agent 错误: {e}"));
        }
    }

    // 清理取消令牌
    agent_state.remove_cancel_token(session_id).await;

    Ok(())
}

/// 停止生成
#[tauri::command]
pub async fn chat_stop_generation(
    agent_state: State<'_, AsterAgentState>,
    session_id: String,
) -> Result<bool, String> {
    tracing::info!("[UnifiedChat] 停止生成: session={}", session_id);
    Ok(agent_state.cancel_session(&session_id).await)
}

/// 配置会话的 Provider
#[tauri::command]
pub async fn chat_configure_provider(
    db: State<'_, DbConnection>,
    agent_state: State<'_, AsterAgentState>,
    session_id: String,
    provider_type: String,
    model: String,
) -> Result<(), String> {
    tracing::info!(
        "[UnifiedChat] 配置 Provider: session={}, provider={}, model={}",
        session_id,
        provider_type,
        model
    );

    // 确保 Agent 已初始化
    agent_state.init_agent_with_db(&db).await?;

    // 配置 Provider
    agent_state
        .configure_provider_from_pool(&db, &provider_type, &model, &session_id)
        .await?;

    Ok(())
}
