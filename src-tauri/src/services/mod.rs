//! 业务服务模块
//!
//! 核心业务逻辑已迁移到 proxycast-services crate。
//! 本模块保留 Tauri 相关服务。

// 保留在主 crate 的 Tauri 相关服务
pub mod file_browser_service;
pub mod sysinfo_service;
pub mod update_check_service;
pub mod update_window;
