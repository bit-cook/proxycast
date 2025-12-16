//! 配置管理模块
//!
//! 提供 YAML 配置文件支持、热重载和配置导入导出功能
//! 同时保持与旧版 JSON 配置的向后兼容性

mod hot_reload;
mod types;
mod yaml;

pub use hot_reload::{
    ConfigChangeEvent, ConfigChangeKind, FileWatcher, HotReloadError, HotReloadManager,
    HotReloadStatus, ReloadResult,
};
pub use types::{
    Config, CustomProviderConfig, InjectionRuleConfig, InjectionSettings, LoggingConfig,
    ProviderConfig, ProvidersConfig, RetrySettings, RoutingConfig, RoutingRuleConfig, ServerConfig,
};
pub use yaml::{load_config, save_config, save_config_yaml, ConfigError, ConfigManager};

#[cfg(test)]
mod tests;
