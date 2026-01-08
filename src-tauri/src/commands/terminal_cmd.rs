//! 终端 Tauri 命令
//!
//! 提供终端核心能力的 Tauri 命令接口。
//!
//! ## 架构说明
//! PTY 在后端预创建，使用默认大小 (24x80)。前端连接后通过 resize 同步实际大小。
//!
//! ## 命令列表
//! - `terminal_create_session` - 创建终端会话（使用默认大小）
//! - `terminal_write` - 向终端发送输入
//! - `terminal_resize` - 调整终端大小
//! - `terminal_close` - 关闭终端会话
//! - `terminal_list_sessions` - 获取所有会话列表

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tokio::sync::RwLock;

use crate::terminal::{SessionMetadata, TerminalSessionManager};

/// 终端会话管理器状态包装
pub struct TerminalManagerState(pub Arc<RwLock<Option<TerminalSessionManager>>>);

/// 创建终端会话响应
#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    /// 会话 ID
    pub session_id: String,
}

/// 创建终端会话（使用默认大小）
///
/// PTY 使用默认大小 (24x80) 预创建，前端连接后通过 resize 同步实际大小。
///
/// # 参数
/// - `cwd`: 工作目录（可选）
///
/// # 返回
/// - `Ok(CreateSessionResponse)`: 包含会话 ID
/// - `Err(String)`: 错误信息
#[tauri::command]
pub async fn terminal_create_session(
    state: State<'_, TerminalManagerState>,
    cwd: Option<String>,
) -> Result<CreateSessionResponse, String> {
    let guard = state.inner().0.read().await;
    let manager = guard
        .as_ref()
        .ok_or_else(|| "终端管理器未初始化".to_string())?;

    let session_id = manager
        .create_session_with_options(24, 80, cwd)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CreateSessionResponse { session_id })
}

/// 向终端发送输入
///
/// # 参数
/// - `session_id`: 会话 ID
/// - `data`: Base64 编码的输入数据
#[tauri::command]
pub async fn terminal_write(
    state: State<'_, TerminalManagerState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let guard = state.inner().0.read().await;
    let manager = guard
        .as_ref()
        .ok_or_else(|| "终端管理器未初始化".to_string())?;

    manager
        .write_to_session_base64(&session_id, &data)
        .await
        .map_err(|e| e.to_string())
}

/// 调整终端大小
///
/// # 参数
/// - `session_id`: 会话 ID
/// - `rows`: 新的行数
/// - `cols`: 新的列数
#[tauri::command]
pub async fn terminal_resize(
    state: State<'_, TerminalManagerState>,
    session_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let guard = state.inner().0.read().await;
    let manager = guard
        .as_ref()
        .ok_or_else(|| "终端管理器未初始化".to_string())?;

    manager
        .resize_session(&session_id, rows, cols)
        .await
        .map_err(|e| e.to_string())
}

/// 关闭终端会话
///
/// # 参数
/// - `session_id`: 会话 ID
#[tauri::command]
pub async fn terminal_close(
    state: State<'_, TerminalManagerState>,
    session_id: String,
) -> Result<(), String> {
    let guard = state.inner().0.read().await;
    let manager = guard
        .as_ref()
        .ok_or_else(|| "终端管理器未初始化".to_string())?;

    manager
        .close_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取所有终端会话
#[tauri::command]
pub async fn terminal_list_sessions(
    state: State<'_, TerminalManagerState>,
) -> Result<Vec<SessionMetadata>, String> {
    let guard = state.inner().0.read().await;
    let manager = guard
        .as_ref()
        .ok_or_else(|| "终端管理器未初始化".to_string())?;

    Ok(manager.list_sessions().await)
}

/// 获取单个终端会话信息
///
/// # 参数
/// - `session_id`: 会话 ID
#[tauri::command]
pub async fn terminal_get_session(
    state: State<'_, TerminalManagerState>,
    session_id: String,
) -> Result<Option<SessionMetadata>, String> {
    let guard = state.inner().0.read().await;
    let manager = guard
        .as_ref()
        .ok_or_else(|| "终端管理器未初始化".to_string())?;

    Ok(manager.get_session(&session_id).await)
}
