//! Tauri 前端通知观察者
//!
//! 将配置变更事件转发到前端

use async_trait::async_trait;
use proxycast_config::observer::events::ConfigChangeEvent;
use proxycast_config::observer::traits::ConfigObserver;
use proxycast_core::config::Config;
use tauri::{AppHandle, Emitter};

/// Tauri 前端通知观察者
pub struct TauriObserver {
    app_handle: AppHandle,
}

#[allow(dead_code)]
impl TauriObserver {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl ConfigObserver for TauriObserver {
    fn name(&self) -> &str {
        "TauriObserver"
    }

    fn priority(&self) -> i32 {
        1000
    }

    async fn on_config_changed(
        &self,
        event: &ConfigChangeEvent,
        _config: &Config,
    ) -> Result<(), String> {
        self.app_handle
            .emit("config-changed-detail", event)
            .map_err(|e| e.to_string())?;

        self.app_handle
            .emit("config-refresh-needed", ())
            .map_err(|e| e.to_string())?;

        tracing::debug!("[TauriObserver] 已通知前端配置变更: {}", event.event_type());

        Ok(())
    }
}
