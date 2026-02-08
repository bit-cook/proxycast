//! 菜单文本格式化模块（桥接层）
//!
//! 纯逻辑已迁移到 `proxycast-core` crate，
//! 本模块保留兼容导出。

pub use proxycast_core::tray_format::{
    format_api_address, format_credential_status, format_request_count, format_server_status,
};
