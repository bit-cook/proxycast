//! Skills 集成模块
//!
//! 纯逻辑已迁移到 `proxycast-skills` crate，
//! 本模块保留 Tauri 相关实现和兼容导出层。

mod execution_callback;
mod llm_provider;

// Tauri 实现（留在主 crate）
pub use execution_callback::TauriExecutionCallback;

// 兼容导出（实际实现位于 proxycast-skills crate）
pub use llm_provider::ProxyCastLlmProvider;
