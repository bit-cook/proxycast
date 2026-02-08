//! ProxyCast Agent Crate
//!
//! 包含 Agent 模块中不依赖主 crate 内部模块的纯逻辑部分。
//! 深耦合部分（aster_state、aster_agent 流式桥接）留在主 crate。

pub mod aster_state;
pub mod aster_state_support;
pub mod credential_bridge;
pub mod event_converter;
pub mod mcp_bridge;
pub mod prompt;
pub mod session_store;
pub mod subagent_scheduler;

pub use aster_state::{AsterAgentState, ProviderConfig};
pub use aster_state_support::{
    build_project_system_prompt, create_proxycast_identity, create_session_config_with_project,
    message_helpers, reload_proxycast_skills, SessionConfigBuilder,
};
pub use credential_bridge::{
    create_aster_provider, AsterProviderConfig, CredentialBridge, CredentialBridgeError,
};
pub use event_converter::{convert_agent_event, convert_to_tauri_message, TauriAgentEvent};
pub use prompt::SystemPromptBuilder;
pub use session_store::{
    create_session_sync, get_session_sync, list_sessions_sync, SessionDetail, SessionInfo,
};
pub use subagent_scheduler::{
    ProxyCastScheduler, ProxyCastSubAgentExecutor, SchedulerEventEmitter, SubAgentProgressEvent,
};
