//! Skills 集成模块
//!
//! 本模块实现 aster-rust Skills 系统与 ProxyCast 的集成。
//!
//! ## 模块结构
//! - `llm_provider`: ProxyCastLlmProvider 实现，使用 ProviderPoolService 调用 LLM
//! - `execution_callback`: TauriExecutionCallback 实现，通过 Tauri 事件发送进度
//!
//! ## 使用示例
//! ```ignore
//! use proxycast::skills::{ProxyCastLlmProvider, TauriExecutionCallback};
//!
//! let provider = ProxyCastLlmProvider::new(pool_service, api_key_service, db);
//! let callback = TauriExecutionCallback::new(app_handle, execution_id);
//! ```

mod execution_callback;
mod llm_provider;
mod skill_loader;

pub use execution_callback::{
    events, ExecutionCallback, ExecutionCompletePayload, StepCompletePayload, StepErrorPayload,
    StepStartPayload, TauriExecutionCallback,
};
pub use llm_provider::{LlmProvider, ProxyCastLlmProvider, SkillError};
pub(crate) use skill_loader::{
    find_skill_by_name, get_proxycast_skills_dir, load_skills_from_directory,
};
#[cfg(test)]
pub(crate) use skill_loader::{
    load_skill_from_file, parse_allowed_tools, parse_boolean, parse_skill_frontmatter,
};
