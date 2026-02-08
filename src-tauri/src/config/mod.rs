//! 配置管理模块
//!
//! 核心配置类型、YAML 支持、热重载和导入导出功能已迁移到 proxycast-core crate。
//! 本模块保留 observer（依赖 Tauri）和集成测试。

// observer 模块保留在主 crate（依赖 Tauri）
pub mod observer;

// 兼容导出：配置核心能力已迁移到 proxycast-core crate
pub use proxycast_core::config::*;

// 重新导出观察者模块的核心类型
pub use observer::ConfigChangeSource;
pub use proxycast_config::observer::manager::GlobalConfigManager;
pub use proxycast_config::GlobalConfigManagerState;

#[cfg(test)]
mod tests;
