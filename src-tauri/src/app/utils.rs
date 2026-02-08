//! 辅助函数（桥接层）
//!
//! 纯逻辑已迁移到 `proxycast-core` crate，
//! 本模块保留兼容导出。

pub use proxycast_core::app_utils::{
    generate_api_key, is_loopback_host, is_non_local_bind, is_valid_bind_host, mask_token,
};
