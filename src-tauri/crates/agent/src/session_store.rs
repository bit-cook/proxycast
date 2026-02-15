//! Agent 会话存储服务
//!
//! 提供会话创建、列表查询、详情查询能力。
//! 数据来源为 ProxyCast 数据库（AgentDao）。

use chrono::Utc;
use proxycast_core::agent::types::{AgentMessage, AgentSession, ContentPart, MessageContent};
use proxycast_core::database::dao::agent::AgentDao;
use proxycast_core::database::DbConnection;
use proxycast_core::workspace::WorkspaceManager;
use uuid::Uuid;

use crate::event_converter::{TauriMessage, TauriMessageContent};

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
    pub messages: Vec<TauriMessage>,
}

/// 解析会话 working_dir（优先入参，其次 workspace_id）
fn resolve_session_working_dir(
    db: &DbConnection,
    working_dir: Option<String>,
    workspace_id: String,
) -> Result<Option<String>, String> {
    if let Some(path) = working_dir {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }

    let workspace_id = workspace_id.trim().to_string();
    if workspace_id.is_empty() {
        return Err("workspace_id 必填，请先选择项目工作区".to_string());
    }

    let manager = WorkspaceManager::new(db.clone());
    if let Some(workspace) = manager.get(&workspace_id)? {
        return Ok(Some(workspace.root_path.to_string_lossy().to_string()));
    }

    Err(format!("Workspace 不存在: {}", workspace_id))
}

/// 创建新会话
pub fn create_session_sync(
    db: &DbConnection,
    name: Option<String>,
    working_dir: Option<String>,
    workspace_id: String,
) -> Result<String, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    let session_name = name.unwrap_or_else(|| "新对话".to_string());
    let session_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    drop(conn);

    let resolved_working_dir = resolve_session_working_dir(db, working_dir, workspace_id)?;

    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let session = AgentSession {
        id: session_id.clone(),
        model: "agent:default".to_string(),
        messages: Vec::new(),
        system_prompt: None,
        title: Some(session_name),
        working_dir: resolved_working_dir,
        created_at: now.clone(),
        updated_at: now,
    };

    AgentDao::create_session(&conn, &session).map_err(|e| format!("创建会话失败: {e}"))?;

    Ok(session_id)
}

/// 列出所有会话
pub fn list_sessions_sync(db: &DbConnection) -> Result<Vec<SessionInfo>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    let sessions = AgentDao::list_sessions(&conn).map_err(|e| format!("获取会话列表失败: {e}"))?;

    Ok(sessions
        .into_iter()
        .map(|session| {
            let messages_count = AgentDao::get_message_count(&conn, &session.id).unwrap_or(0);
            SessionInfo {
                id: session.id,
                name: session.title.unwrap_or_else(|| "未命名".to_string()),
                created_at: chrono::DateTime::parse_from_rfc3339(&session.created_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
                updated_at: chrono::DateTime::parse_from_rfc3339(&session.updated_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
                messages_count,
            }
        })
        .collect())
}

/// 获取会话详情
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
            .map(|message| convert_agent_message(&message))
            .collect(),
    })
}

/// 重命名会话
pub fn rename_session_sync(db: &DbConnection, session_id: &str, name: &str) -> Result<(), String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("会话名称不能为空".to_string());
    }

    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    AgentDao::update_title(&conn, session_id, trimmed_name)
        .map_err(|e| format!("更新会话标题失败: {e}"))?;

    let now = Utc::now().to_rfc3339();
    AgentDao::update_session_time(&conn, session_id, &now)
        .map_err(|e| format!("更新会话时间失败: {e}"))?;

    Ok(())
}

/// 删除会话
pub fn delete_session_sync(db: &DbConnection, session_id: &str) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    AgentDao::delete_session(&conn, session_id).map_err(|e| format!("删除会话失败: {e}"))?;
    Ok(())
}

/// 将 AgentMessage 转换为 TauriMessage
fn convert_agent_message(message: &AgentMessage) -> TauriMessage {
    let content = match &message.content {
        MessageContent::Text(text) => vec![TauriMessageContent::Text { text: text.clone() }],
        MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| {
                if let ContentPart::Text { text } = part {
                    Some(TauriMessageContent::Text { text: text.clone() })
                } else {
                    None
                }
            })
            .collect(),
    };

    let timestamp = chrono::DateTime::parse_from_rfc3339(&message.timestamp)
        .map(|dt| dt.timestamp())
        .unwrap_or(0);

    TauriMessage {
        id: None,
        role: message.role.clone(),
        content,
        timestamp,
    }
}
