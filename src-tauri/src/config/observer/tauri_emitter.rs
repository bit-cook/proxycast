//! Tauri 配置事件发射器
//!
//! 实现 ConfigEventEmit trait，通过 Tauri AppHandle 发送事件

use proxycast_config::ConfigChangeEvent;
use proxycast_config::ConfigEventEmit;
use tauri::{AppHandle, Emitter};

/// Tauri 配置事件发射器
pub struct TauriConfigEmitter {
    app_handle: AppHandle,
}

impl TauriConfigEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl ConfigEventEmit for TauriConfigEmitter {
    fn emit_config_event(
        &self,
        event_name: &str,
        payload: &ConfigChangeEvent,
    ) -> Result<(), String> {
        self.app_handle
            .emit(event_name, payload)
            .map_err(|e| e.to_string())
    }

    fn emit_empty_event(&self, event_name: &str) -> Result<(), String> {
        self.app_handle
            .emit(event_name, ())
            .map_err(|e| e.to_string())
    }
}
