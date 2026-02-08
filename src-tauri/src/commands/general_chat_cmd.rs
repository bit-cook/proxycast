//! 通用对话 Tauri 命令模块
//!
//! 提供通用对话功能的前端调用接口
//!
//! ## 主要命令
//! - `general_chat_create_session` - 创建新会话
//! - `general_chat_list_sessions` - 获取会话列表
//! - `general_chat_get_session` - 获取会话详情
//! - `general_chat_delete_session` - 删除会话
//! - `general_chat_rename_session` - 重命名会话
//! - `general_chat_send_message` - 发送消息（流式响应）
//! - `general_chat_stop_generation` - 停止生成
//! - `general_chat_get_messages` - 获取消息列表

use crate::database::dao::general_chat::GeneralChatDao;
use crate::database::DbConnection;
use proxycast_services::general_chat::{
    ChatMessage, ChatSession, ContentBlock, MessageRole, SessionDetail,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

// ==================== 会话管理命令 ====================

/// 创建新会话
///
/// # Arguments
/// * `name` - 会话名称（可选，默认为"新对话"）
/// * `metadata` - 额外元数据（可选）
#[tauri::command]
pub async fn general_chat_create_session(
    db: State<'_, DbConnection>,
    name: Option<String>,
    metadata: Option<serde_json::Value>,
) -> Result<ChatSession, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let session = ChatSession {
        id: Uuid::new_v4().to_string(),
        name: name.unwrap_or_else(|| "新对话".to_string()),
        created_at: now,
        updated_at: now,
        metadata,
    };

    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    GeneralChatDao::create_session(&conn, &session).map_err(|e| format!("创建会话失败: {e}"))?;

    tracing::info!(
        "[GeneralChat] 创建会话: id={}, name={}",
        session.id,
        session.name
    );
    Ok(session)
}

/// 获取会话列表
#[tauri::command]
pub async fn general_chat_list_sessions(
    db: State<'_, DbConnection>,
) -> Result<Vec<ChatSession>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    let sessions =
        GeneralChatDao::list_sessions(&conn).map_err(|e| format!("获取会话列表失败: {e}"))?;

    Ok(sessions)
}

/// 获取会话详情（包含消息列表）
///
/// # Arguments
/// * `session_id` - 会话 ID
/// * `message_limit` - 消息数量限制（可选）
#[tauri::command]
pub async fn general_chat_get_session(
    db: State<'_, DbConnection>,
    session_id: String,
    message_limit: Option<i32>,
) -> Result<SessionDetail, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let session = GeneralChatDao::get_session(&conn, &session_id)
        .map_err(|e| format!("获取会话失败: {e}"))?
        .ok_or_else(|| "会话不存在".to_string())?;

    let messages = GeneralChatDao::get_messages(&conn, &session_id, message_limit, None)
        .map_err(|e| format!("获取消息失败: {e}"))?;

    let message_count = GeneralChatDao::get_message_count(&conn, &session_id)
        .map_err(|e| format!("获取消息数量失败: {e}"))?;

    Ok(SessionDetail {
        session,
        messages,
        message_count,
    })
}

/// 删除会话
///
/// # Arguments
/// * `session_id` - 会话 ID
#[tauri::command]
pub async fn general_chat_delete_session(
    db: State<'_, DbConnection>,
    session_id: String,
) -> Result<bool, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let deleted = GeneralChatDao::delete_session(&conn, &session_id)
        .map_err(|e| format!("删除会话失败: {e}"))?;

    if deleted {
        tracing::info!("[GeneralChat] 删除会话: id={}", session_id);
    }

    Ok(deleted)
}

/// 重命名会话
///
/// # Arguments
/// * `session_id` - 会话 ID
/// * `name` - 新名称
#[tauri::command]
pub async fn general_chat_rename_session(
    db: State<'_, DbConnection>,
    session_id: String,
    name: String,
) -> Result<bool, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let renamed = GeneralChatDao::rename_session(&conn, &session_id, &name)
        .map_err(|e| format!("重命名会话失败: {e}"))?;

    if renamed {
        tracing::info!("[GeneralChat] 重命名会话: id={}, name={}", session_id, name);
    }

    Ok(renamed)
}

// ==================== 消息管理命令 ====================

/// 获取会话消息列表
///
/// # Arguments
/// * `session_id` - 会话 ID
/// * `limit` - 消息数量限制（可选）
/// * `before_id` - 在此消息 ID 之前的消息（用于分页）
#[tauri::command]
pub async fn general_chat_get_messages(
    db: State<'_, DbConnection>,
    session_id: String,
    limit: Option<i32>,
    before_id: Option<String>,
) -> Result<Vec<ChatMessage>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let messages = GeneralChatDao::get_messages(&conn, &session_id, limit, before_id.as_deref())
        .map_err(|e| format!("获取消息失败: {e}"))?;

    Ok(messages)
}

/// 添加消息到会话
///
/// # Arguments
/// * `session_id` - 会话 ID
/// * `role` - 消息角色 (user/assistant/system)
/// * `content` - 消息内容
/// * `blocks` - 内容块列表（可选）
/// * `metadata` - 额外元数据（可选）
#[tauri::command]
pub async fn general_chat_add_message(
    db: State<'_, DbConnection>,
    session_id: String,
    role: String,
    content: String,
    blocks: Option<Vec<ContentBlock>>,
    metadata: Option<serde_json::Value>,
) -> Result<ChatMessage, String> {
    let now = chrono::Utc::now().timestamp_millis();

    let message_role = match role.as_str() {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        _ => return Err(format!("无效的消息角色: {role}")),
    };

    let message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.clone(),
        role: message_role,
        content,
        blocks,
        status: "complete".to_string(),
        created_at: now,
        metadata,
    };

    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    // 检查会话是否存在
    if !GeneralChatDao::session_exists(&conn, &session_id)
        .map_err(|e| format!("检查会话失败: {e}"))?
    {
        return Err("会话不存在".to_string());
    }

    GeneralChatDao::add_message(&conn, &message).map_err(|e| format!("添加消息失败: {e}"))?;

    tracing::debug!(
        "[GeneralChat] 添加消息: session={}, role={:?}, len={}",
        session_id,
        message.role,
        message.content.len()
    );

    Ok(message)
}

// ==================== 流式消息命令 ====================

use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

/// 流式消息事件
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum StreamEvent {
    /// 开始生成
    Start { message_id: String },
    /// 文本增量
    Delta { content: String },
    /// 生成完成
    Done { message_id: String, content: String },
    /// 发生错误
    Error { message: String },
}

/// 流式消息请求
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    /// 会话 ID
    pub session_id: String,
    /// 用户消息内容
    pub content: String,
    /// 事件名称（用于前端监听）
    pub event_name: String,
    /// Provider 配置（可选）
    #[serde(default)]
    #[allow(dead_code)]
    pub provider: Option<String>,
    /// 模型名称（可选）
    #[serde(default)]
    #[allow(dead_code)]
    pub model: Option<String>,
}

/// 全局停止标志存储
static STOP_FLAGS: once_cell::sync::Lazy<Arc<RwLock<std::collections::HashMap<String, bool>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(std::collections::HashMap::new())));

/// 发送消息并获取流式响应
///
/// 此命令会：
/// 1. 保存用户消息到数据库
/// 2. 调用 AI Provider 获取响应
/// 3. 通过事件流式返回响应
/// 4. 保存 AI 响应到数据库
#[tauri::command]
pub async fn general_chat_send_message(
    app: AppHandle,
    db: State<'_, DbConnection>,
    request: SendMessageRequest,
) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let user_message_id = Uuid::new_v4().to_string();
    let assistant_message_id = Uuid::new_v4().to_string();

    // 保存用户消息
    let user_message = ChatMessage {
        id: user_message_id.clone(),
        session_id: request.session_id.clone(),
        role: MessageRole::User,
        content: request.content.clone(),
        blocks: None,
        status: "complete".to_string(),
        created_at: now,
        metadata: None,
    };

    {
        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

        // 检查会话是否存在
        if !GeneralChatDao::session_exists(&conn, &request.session_id)
            .map_err(|e| format!("检查会话失败: {e}"))?
        {
            return Err("会话不存在".to_string());
        }

        GeneralChatDao::add_message(&conn, &user_message)
            .map_err(|e| format!("保存用户消息失败: {e}"))?;
    }

    // 设置停止标志
    {
        let mut flags = STOP_FLAGS.write().await;
        flags.insert(request.session_id.clone(), false);
    }

    // 发送开始事件
    let start_event = StreamEvent::Start {
        message_id: assistant_message_id.clone(),
    };
    if let Err(e) = app.emit(&request.event_name, &start_event) {
        tracing::error!("[GeneralChat] 发送开始事件失败: {}", e);
    }

    // TODO: 实际调用 AI Provider 获取响应
    // 这里先返回一个模拟响应，后续集成 Provider 系统
    let mock_response = format!(
        "这是对「{}」的模拟响应。实际实现需要集成 Provider 系统。",
        request.content
    );

    // 模拟流式输出
    for chunk in mock_response.chars().collect::<Vec<_>>().chunks(5) {
        // 检查是否需要停止
        {
            let flags = STOP_FLAGS.read().await;
            if flags.get(&request.session_id).copied().unwrap_or(false) {
                tracing::info!("[GeneralChat] 生成被用户停止");
                break;
            }
        }

        let content: String = chunk.iter().collect();
        let delta_event = StreamEvent::Delta { content };
        if let Err(e) = app.emit(&request.event_name, &delta_event) {
            tracing::error!("[GeneralChat] 发送增量事件失败: {}", e);
        }

        // 模拟延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // 保存 AI 响应消息
    let assistant_message = ChatMessage {
        id: assistant_message_id.clone(),
        session_id: request.session_id.clone(),
        role: MessageRole::Assistant,
        content: mock_response.clone(),
        blocks: None,
        status: "complete".to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
        metadata: None,
    };

    {
        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
        GeneralChatDao::add_message(&conn, &assistant_message)
            .map_err(|e| format!("保存 AI 响应失败: {e}"))?;
    }

    // 发送完成事件
    let done_event = StreamEvent::Done {
        message_id: assistant_message_id.clone(),
        content: mock_response,
    };
    if let Err(e) = app.emit(&request.event_name, &done_event) {
        tracing::error!("[GeneralChat] 发送完成事件失败: {}", e);
    }

    // 清理停止标志
    {
        let mut flags = STOP_FLAGS.write().await;
        flags.remove(&request.session_id);
    }

    Ok(assistant_message_id)
}

/// 停止生成
///
/// # Arguments
/// * `session_id` - 会话 ID
#[tauri::command]
pub async fn general_chat_stop_generation(session_id: String) -> Result<bool, String> {
    tracing::info!("[GeneralChat] 停止生成: session={}", session_id);

    let mut flags = STOP_FLAGS.write().await;
    if let std::collections::hash_map::Entry::Occupied(mut e) = flags.entry(session_id) {
        e.insert(true);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// 自动生成会话标题请求
#[derive(Debug, Deserialize)]
pub struct GenerateTitleRequest {
    /// 会话 ID
    pub session_id: String,
    /// 用户第一条消息内容
    pub first_message: String,
    /// Provider 名称（可选，暂未使用，预留给未来支持多 provider）
    #[serde(default)]
    pub provider: Option<String>,
    /// 模型名称（可选，用于指定生成标题的模型）
    #[serde(default)]
    pub model: Option<String>,
}

/// 自动生成会话标题
///
/// 基于用户第一条消息，调用 AI 生成简短的会话标题
///
/// # Arguments
/// * `request` - 生成标题请求
#[tauri::command]
pub async fn general_chat_generate_title(
    db: State<'_, DbConnection>,
    request: GenerateTitleRequest,
) -> Result<String, String> {
    tracing::info!(
        "[GeneralChat] 生成标题: session={}, message_len={}, provider={:?}, model={:?}",
        request.session_id,
        request.first_message.len(),
        request.provider,
        request.model
    );

    // 生成标题的 prompt
    let prompt = format!(
        "请根据以下用户消息，生成一个简短的对话标题（不超过15个字符，不要使用引号，直接输出标题）：\n\n{}",
        request.first_message.chars().take(500).collect::<String>()
    );

    // 尝试调用 AI 生成标题，使用指定的模型或默认模型
    let model = request.model.as_deref();
    let title = match generate_title_with_ai(&prompt, model).await {
        Ok(ai_title) => {
            tracing::info!("[GeneralChat] AI 生成标题成功: {}", ai_title);
            // 清理 AI 返回的标题（去除引号、换行等）
            clean_title(&ai_title)
        }
        Err(e) => {
            tracing::warn!("[GeneralChat] AI 生成标题失败，使用 fallback: {}", e);
            // Fallback: 使用简单的截取逻辑
            generate_fallback_title(&request.first_message)
        }
    };

    // 更新数据库中的会话标题
    {
        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
        GeneralChatDao::rename_session(&conn, &request.session_id, &title)
            .map_err(|e| format!("更新标题失败: {e}"))?;
    }

    tracing::info!(
        "[GeneralChat] 标题生成完成: session={}, title={}",
        request.session_id,
        title
    );

    Ok(title)
}

/// 使用 AI 生成标题
async fn generate_title_with_ai(prompt: &str, model: Option<&str>) -> Result<String, String> {
    use crate::models::openai::{ChatCompletionRequest, ChatMessage, MessageContent};
    use crate::providers::openai_custom::OpenAICustomProvider;

    // 使用本地代理服务器调用 AI
    // 这样可以利用已配置的凭证池
    let provider = OpenAICustomProvider::with_config(
        "local".to_string(),
        Some("http://127.0.0.1:5678".to_string()),
    );

    let request = ChatCompletionRequest {
        model: model.unwrap_or("default").to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(MessageContent::Text(prompt.to_string())),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        temperature: Some(0.3),
        max_tokens: Some(32),
        top_p: None,
        stream: false,
        tools: None,
        tool_choice: None,
        reasoning_effort: None,
    };

    let resp = provider
        .call_api(&request)
        .await
        .map_err(|e| format!("API 调用失败: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("API 返回错误: {status} - {body}"));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("解析响应失败: {e}"))?;

    let content = parsed["choices"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["message"]["content"].as_str())
        .unwrap_or("")
        .to_string();

    if content.is_empty() {
        return Err("AI 返回空内容".to_string());
    }

    Ok(content)
}

/// 清理 AI 生成的标题
fn clean_title(title: &str) -> String {
    let cleaned = title
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('「')
        .trim_matches('」')
        .trim_matches('《')
        .trim_matches('》')
        .lines()
        .next()
        .unwrap_or(title)
        .trim();

    // 限制长度
    if cleaned.chars().count() > 20 {
        format!("{}...", cleaned.chars().take(17).collect::<String>())
    } else {
        cleaned.to_string()
    }
}

/// 生成 fallback 标题
fn generate_fallback_title(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.chars().count() > 20 {
        format!("{}...", trimmed.chars().take(17).collect::<String>())
    } else {
        trimmed.to_string()
    }
}
