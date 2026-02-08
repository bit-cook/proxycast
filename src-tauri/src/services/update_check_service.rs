//! 更新检查服务（桥接层）
//!
//! 纯逻辑已迁移到 `proxycast-services` crate，
//! 本模块仅保留兼容导出。

pub use proxycast_services::update_check_service::{
    UpdateCheckService, UpdateCheckServiceState, UpdateCheckState, UpdateInfo,
};
