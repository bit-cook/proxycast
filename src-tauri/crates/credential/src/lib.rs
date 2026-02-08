//! 凭证池管理 crate
//!
//! 提供负载均衡、配额管理和凭证同步功能
//!
//! ## 模块结构
//!
//! - `balancer` - 负载均衡策略（轮询、最少使用、随机）
//! - `quota` - 配额超限检测、自动切换和冷却恢复
//! - `sync` - 凭证与 YAML 配置文件的同步

mod balancer;
mod quota;
mod sync;

// 重新导出
pub use balancer::{BalanceStrategy, CooldownInfo, CredentialSelection, LoadBalancer};
pub use quota::{
    create_shared_quota_manager, start_quota_cleanup_task, AllCredentialsExhaustedError,
    QuotaAutoSwitchResult, QuotaExceededRecord, QuotaManager,
};
pub use sync::{CredentialSyncService, SyncError};
