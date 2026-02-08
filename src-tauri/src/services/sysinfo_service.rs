//! 系统信息服务（Tauri 命令桥接层）
//!
//! 纯逻辑已迁移到 `proxycast-services` crate，
//! 本模块仅保留 Tauri 命令封装。

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

pub use proxycast_services::sysinfo_service::{get_sysinfo_service, SysinfoEmitter};
pub use proxycast_services::sysinfo_service::{SysinfoData, SysinfoService};

/// Tauri 命令：获取当前系统信息
#[tauri::command]
pub async fn get_sysinfo() -> Result<SysinfoData, String> {
    proxycast_services::sysinfo_service::get_sysinfo().await
}

/// Tauri 命令：开始订阅系统信息
/// 每秒向前端发送 sysinfo 事件
#[tauri::command]
pub async fn subscribe_sysinfo(app: AppHandle) -> Result<(), String> {
    let emitter: SysinfoEmitter = Arc::new(move |data: &SysinfoData| {
        app.emit("sysinfo", data)
            .map_err(|e| format!("发送系统信息事件失败: {e}"))
    });

    proxycast_services::sysinfo_service::subscribe_sysinfo(emitter).await
}

/// Tauri 命令：停止订阅系统信息
#[tauri::command]
pub async fn unsubscribe_sysinfo() -> Result<(), String> {
    proxycast_services::sysinfo_service::unsubscribe_sysinfo().await
}
