//! 上下文记忆管理相关的 Tauri 命令

use crate::services::context_memory_service::{
    ContextMemoryService, MemoryEntry, MemoryFileType, MemoryStats,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tracing::{debug, info};

pub struct ContextMemoryServiceState(pub Arc<ContextMemoryService>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMemoryRequest {
    pub session_id: String,
    pub file_type: MemoryFileType,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordErrorRequest {
    pub session_id: String,
    pub error_description: String,
    pub attempted_solution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveErrorRequest {
    pub session_id: String,
    pub error_description: String,
    pub resolution: String,
}

#[tauri::command]
pub async fn save_memory_entry(
    memory_service: State<'_, ContextMemoryServiceState>,
    request: SaveMemoryRequest,
) -> Result<(), String> {
    debug!(
        "保存记忆条目: {} (会话: {})",
        request.title, request.session_id
    );

    let entry = MemoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: request.session_id.clone(),
        file_type: request.file_type,
        title: request.title,
        content: request.content,
        tags: request.tags,
        priority: request.priority,
        created_at: chrono::Utc::now().timestamp_millis(),
        updated_at: chrono::Utc::now().timestamp_millis(),
        archived: false,
    };

    memory_service.0.save_memory_entry(&entry)?;
    info!("记忆条目保存成功: {}", entry.title);
    Ok(())
}

#[tauri::command]
pub async fn get_session_memories(
    memory_service: State<'_, ContextMemoryServiceState>,
    session_id: String,
    file_type: Option<MemoryFileType>,
) -> Result<Vec<MemoryEntry>, String> {
    debug!("获取会话记忆: {} (类型: {:?})", session_id, file_type);
    let memories = memory_service
        .0
        .get_session_memories(&session_id, file_type)?;
    info!("获取到 {} 个记忆条目", memories.len());
    Ok(memories)
}

#[tauri::command]
pub async fn get_memory_context(
    memory_service: State<'_, ContextMemoryServiceState>,
    session_id: String,
) -> Result<String, String> {
    debug!("获取记忆上下文: {}", session_id);
    let context = memory_service.0.get_memory_context(&session_id)?;
    info!("记忆上下文长度: {} 字符", context.len());
    Ok(context)
}

#[tauri::command]
pub async fn record_error(
    memory_service: State<'_, ContextMemoryServiceState>,
    request: RecordErrorRequest,
) -> Result<(), String> {
    debug!(
        "记录错误: {} (会话: {})",
        request.error_description, request.session_id
    );
    memory_service.0.record_error(
        &request.session_id,
        &request.error_description,
        &request.attempted_solution,
    )?;
    info!("错误记录成功");
    Ok(())
}

#[tauri::command]
pub async fn should_avoid_operation(
    memory_service: State<'_, ContextMemoryServiceState>,
    session_id: String,
    operation_description: String,
) -> Result<bool, String> {
    debug!(
        "检查是否避免操作: {} (会话: {})",
        operation_description, session_id
    );
    let should_avoid = memory_service
        .0
        .should_avoid_operation(&session_id, &operation_description);
    if should_avoid {
        info!("建议避免操作: {}", operation_description);
    }
    Ok(should_avoid)
}

#[tauri::command]
pub async fn mark_error_resolved(
    memory_service: State<'_, ContextMemoryServiceState>,
    request: ResolveErrorRequest,
) -> Result<(), String> {
    debug!(
        "标记错误已解决: {} (会话: {})",
        request.error_description, request.session_id
    );
    memory_service.0.mark_error_resolved(
        &request.session_id,
        &request.error_description,
        &request.resolution,
    )?;
    info!("错误已标记为解决");
    Ok(())
}

#[tauri::command]
pub async fn get_memory_stats(
    memory_service: State<'_, ContextMemoryServiceState>,
    session_id: String,
) -> Result<MemoryStats, String> {
    debug!("获取记忆统计: {}", session_id);
    let stats = memory_service.0.get_memory_stats(&session_id)?;
    info!(
        "记忆统计: {} 个活跃记忆, {} 个未解决错误",
        stats.active_memories, stats.unresolved_errors
    );
    Ok(stats)
}

#[tauri::command]
pub async fn cleanup_expired_memories(
    memory_service: State<'_, ContextMemoryServiceState>,
) -> Result<(), String> {
    debug!("清理过期记忆");
    memory_service.0.cleanup_expired_memories()?;
    info!("过期记忆清理完成");
    Ok(())
}
