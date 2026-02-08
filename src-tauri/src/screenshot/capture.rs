//! 屏幕截图服务（桥接层）
//!
//! 纯逻辑已迁移到 `proxycast-services` crate，
//! 本模块保留兼容导出。

pub use proxycast_services::screenshot_capture_service::{
    cleanup_temp_file, start_capture, CaptureError, CaptureResult,
};
