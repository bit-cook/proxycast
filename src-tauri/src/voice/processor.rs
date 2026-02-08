//! 语音处理器（桥接层）
//!
//! 纯逻辑已迁移到 `proxycast-services` crate，
//! 本模块保留兼容导出。

pub use proxycast_services::voice_processor_service::{polish_text, process_text};
