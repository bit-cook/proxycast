//! 服务器控制命令
//!
//! 包含服务器启动、停止、状态查询等命令。

use crate::app::types::{AppState, LogState};
use crate::app::TokenCacheServiceState;
use crate::commands::provider_pool_cmd::ProviderPoolServiceState;
use crate::commands::telemetry_cmd::TelemetryState;
use crate::database;
use proxycast_server as server;

/// 启动服务器
#[tauri::command]
pub async fn start_server(
    state: tauri::State<'_, AppState>,
    logs: tauri::State<'_, LogState>,
    db: tauri::State<'_, database::DbConnection>,
    pool_service: tauri::State<'_, ProviderPoolServiceState>,
    token_cache: tauri::State<'_, TokenCacheServiceState>,
) -> Result<String, String> {
    let mut s = state.write().await;
    logs.write().await.add("info", "Starting server...");
    s.start(
        logs.inner().clone(),
        pool_service.0.clone(),
        token_cache.0.clone(),
        Some(db.inner().clone()),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 使用 status() 获取实际使用的地址（可能已经自动切换到有效的 IP）
    let status = s.status();
    logs.write().await.add(
        "info",
        &format!("Server started on {}:{}", status.host, status.port),
    );
    Ok("Server started".to_string())
}

/// 停止服务器
#[tauri::command]
pub async fn stop_server(
    state: tauri::State<'_, AppState>,
    logs: tauri::State<'_, LogState>,
) -> Result<String, String> {
    let mut s = state.write().await;
    s.stop().await;
    logs.write().await.add("info", "Server stopped");
    Ok("Server stopped".to_string())
}

/// 获取服务器状态
#[tauri::command]
pub async fn get_server_status(
    state: tauri::State<'_, AppState>,
    telemetry_state: tauri::State<'_, TelemetryState>,
) -> Result<server::ServerStatus, String> {
    let s = state.read().await;
    let mut status = s.status();

    // 从遥测系统获取真实的请求计数
    let stats = telemetry_state.stats.read();
    let summary = stats.summary(None);
    status.requests = summary.total_requests;

    Ok(status)
}
