//! 配置观察者模块
//!
//! 核心逻辑已迁移到 proxycast-config crate。
//! 本模块保留 Tauri 相关实现和重新导出。

mod tauri_emitter;
mod tauri_observer;

// 从 proxycast-config crate 重新导出所有类型
pub use proxycast_config::observer::emitter::{ConfigEventEmit, NoOpEmitter};
pub use proxycast_config::observer::events::{
    AmpConfigChangeEvent, ConfigChangeEvent, ConfigChangeSource, CredentialPoolChangeEvent,
    EndpointProvidersChangeEvent, FullReloadEvent, InjectionChangeEvent, LoggingChangeEvent,
    NativeAgentChangeEvent, RetryChangeEvent, RoutingChangeEvent, ServerChangeEvent,
};
pub use proxycast_config::observer::manager::GlobalConfigManager;
pub use proxycast_config::observer::observers::{
    DefaultProviderRefObserver, EndpointObserver, InjectorObserver, LoggingObserver, RouterObserver,
};
pub use proxycast_config::observer::subject::{
    ConfigSubject, CONFIG_CHANGED_EVENT, CONFIG_RELOAD_EVENT,
};
pub use proxycast_config::observer::traits::{
    ConfigObserver, FnObserver, SyncConfigObserver, SyncObserverWrapper,
};
pub use proxycast_config::GlobalConfigManagerState;

// Tauri 相关实现
pub use tauri_emitter::TauriConfigEmitter;
pub use tauri_observer::TauriObserver;
