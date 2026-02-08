//! 凭证池管理模块
//!
//! 提供多凭证管理、负载均衡和健康检查功能
//!
//! ## 模块结构
//!
//! - `types` - 凭证相关类型定义（来自 proxycast-core）
//! - `pool` - 凭证池管理（来自 proxycast-core）
//! - `health` - 健康检查（来自 proxycast-core）
//! - `risk` - 风控模块（来自 proxycast-core）
//! - `balancer` - 负载均衡策略（来自 proxycast-credential）
//! - `quota` - 配额管理（来自 proxycast-credential）
//! - `sync` - 数据库同步（来自 proxycast-credential）

// 从 proxycast-core 重新导出核心类型模块
pub use proxycast_core::credential::{health, pool, risk, types};

// 重新导出 core 类型
pub use proxycast_core::credential::{
    CooldownConfig, Credential, CredentialData, CredentialPool, CredentialStats, CredentialStatus,
    HealthCheckConfig, HealthCheckResult, HealthChecker, HealthStatus, PoolError, PoolStatus,
    RateLimitEvent, RateLimitStats, RiskController, RiskLevel,
};

// 从 proxycast-credential crate 重新导出
pub use proxycast_credential::*;

#[cfg(test)]
mod tests;
