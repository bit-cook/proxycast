//! 插件系统模块
//!
//! 提供插件扩展功能，支持：
//! - 插件加载和初始化
//! - 请求前/响应后钩子
//! - 插件隔离和错误处理
//! - 插件配置管理
//! - 二进制组件下载和管理
//! - 声明式插件 UI 系统
//! - 插件安装和卸载

pub mod binary_downloader;
pub mod examples;
pub mod installer;
mod loader;
mod manager;
mod task;
mod types;
pub mod ui_builder;
pub mod ui_trait;
pub mod ui_types;

pub use binary_downloader::BinaryDownloader;
pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use task::{
    PluginQueueStats, PluginTaskError, PluginTaskEventPayload, PluginTaskFailure, PluginTaskPolicy,
    PluginTaskRecord, PluginTaskState, PluginTaskTracker,
};
pub use types::{
    BinaryComponentStatus, BinaryManifest, HookResult, PlatformBinaries, Plugin, PluginConfig,
    PluginContext, PluginError, PluginInfo, PluginManifest, PluginState, PluginStatus, PluginType,
};
pub use ui_trait::{NoUI, PluginUI};
pub use ui_types::{
    Action, BoundValue, ChildrenDef, ComponentDef, ComponentType, DataEntry, DataModelUpdate,
    SurfaceDefinition, SurfaceUpdate, UIMessage, UserAction,
};

#[cfg(test)]
mod tests;
