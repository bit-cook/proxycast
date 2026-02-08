//! 终端模块
//!
//! 实际实现位于 `proxycast-terminal` crate。
//! 本模块提供 `TauriEmitter` newtype 桥接 Tauri 与终端 crate。

use std::path::PathBuf;

use tauri::{Emitter, Manager};

use proxycast_terminal::emitter::TerminalEventEmit;

/// Tauri AppHandle 的 newtype 包装
///
/// 实现 `TerminalEventEmit` trait，桥接 Tauri 框架与终端 crate。
#[derive(Clone)]
pub struct TauriEmitter(pub tauri::AppHandle);

impl TerminalEventEmit for TauriEmitter {
    fn emit_event(&self, event: &str, payload: &serde_json::Value) -> Result<(), String> {
        self.0
            .emit(event, payload.clone())
            .map_err(|e| format!("Tauri emit 失败: {e}"))
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        self.0
            .path()
            .app_data_dir()
            .map_err(|e| format!("获取应用数据目录失败: {e}"))
    }
}
