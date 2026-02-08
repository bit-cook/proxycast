//! 配置事件发射器 Trait
//!
//! 抽象 Tauri AppHandle 的 emit 功能，
//! 使 observer 模块不直接依赖 Tauri。

use super::events::ConfigChangeEvent;

/// 配置事件发射器 Trait
///
/// 用于向前端发送配置变更事件。
/// 主 crate 中通过 Tauri AppHandle 实现此 trait。
pub trait ConfigEventEmit: Send + Sync {
    /// 发送配置变更事件
    fn emit_config_event(
        &self,
        event_name: &str,
        payload: &ConfigChangeEvent,
    ) -> Result<(), String>;

    /// 发送无负载事件
    fn emit_empty_event(&self, event_name: &str) -> Result<(), String>;
}

/// 空操作发射器（用于测试）
pub struct NoOpEmitter;

impl ConfigEventEmit for NoOpEmitter {
    fn emit_config_event(
        &self,
        _event_name: &str,
        _payload: &ConfigChangeEvent,
    ) -> Result<(), String> {
        Ok(())
    }

    fn emit_empty_event(&self, _event_name: &str) -> Result<(), String> {
        Ok(())
    }
}
