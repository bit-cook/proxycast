//! 插件系统模块
//!
//! 核心逻辑从 proxycast-core 重新导出，
//! ui_events 依赖 Tauri 保留在主 crate。

// 核心插件能力从 core crate 导出
pub use proxycast_core::plugin::*;

// Tauri 依赖的 UI 事件模块保留在主 crate
pub mod ui_events;
pub use ui_events::{PluginUIEmitter, PluginUIEmitterState, PluginUIEventPayload};
