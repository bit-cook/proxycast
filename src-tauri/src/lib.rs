//! ProxyCast - AI API 代理服务
//!
//! 这是一个 Tauri 应用，提供 AI API 的代理和管理功能。
//!
//! ## Workspace 结构（方案 A - 最小化拆分）
//!
//! 采用最小化拆分策略，只迁移真正独立的模块：
//! - ✅ proxycast-core crate（models, data, logger）
//! - ✅ proxycast-infra crate（proxy, resilience, injection, telemetry）
//! - 主 crate 保留所有业务逻辑模块（包括 plugin，因依赖 Tauri）

// 抑制 objc crate 宏内部的 unexpected_cfgs 警告
// 该警告来自 cocoa/objc 依赖的 msg_send! 宏，是已知的 issue
#![allow(unexpected_cfgs)]

// 重新导出子 crate 的类型
// 注意：主 crate 保留了自己的 data, logger, models 模块，所以只导出 core 的具体类型
pub use proxycast_core::{LogEntry, LogStore, LogStoreConfig, SharedLogStore};
// infra crate 的类型通过 proxycast_infra 前缀访问，避免与 core 的 InjectionMode/InjectionRule 冲突
pub use proxycast_infra::{
    injection, proxy, resilience, telemetry, Failover, FailoverConfig, InjectionConfig,
    InjectionMode, InjectionResult, InjectionRule, Injector, LogRotationConfig, LoggerError,
    ModelStats, ModelTokenStats, PeriodTokenStats, ProviderStats, ProviderTokenStats,
    ProxyClientFactory, ProxyError, ProxyProtocol, RequestLog, RequestLogger, RequestStatus,
    Retrier, RetryConfig, StatsAggregator, StatsSummary, TimeRange, TimeoutConfig,
    TimeoutController, TokenSource, TokenStatsSummary, TokenTracker, TokenUsageRecord,
};

// 核心模块
pub mod agent;
pub mod app;
pub mod backends;
pub mod browser_interceptor;
pub mod connect;
pub mod content;
pub mod credential;
pub mod database;
pub mod flow_monitor;
pub mod memory;
pub mod orchestrator;
pub mod plugin;
pub mod screenshot;
pub mod services;
pub mod session;
pub mod session_files;
pub mod stream;
pub mod terminal;
pub mod translator;
pub mod tray;
pub mod voice;
pub mod workspace;

// 内部模块
mod commands;
mod config;
mod converter;
mod data;
#[cfg(debug_assertions)]
mod dev_bridge;
mod errors;
mod logger;
mod models;
mod providers;
mod server_utils;

// 服务器相关模块
mod middleware;
mod processor;
mod router;
mod server;
mod streaming;
mod websocket;

// 重新导出核心类型以保持向后兼容
pub use app::{AppState, LogState, ProviderType, TokenCacheServiceState, TrayManagerState};
pub use services::provider_pool_service::ProviderPoolService;

// 重新导出 run 函数
pub use app::run;
