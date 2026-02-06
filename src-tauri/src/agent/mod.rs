//! AI Agent 集成模块
//!
//! 基于 aster-rust 框架实现 Agent 功能
//!
//! ## 架构设计
//! - aster_state - Aster Agent 状态管理
//! - aster_agent - Aster Agent 包装器
//! - event_converter - Aster 事件转换器
//! - credential_bridge - 凭证池桥接（连接 ProxyCast 凭证池与 Aster Provider）
//! - subagent_scheduler - SubAgent 调度器集成

pub mod aster_agent;
pub mod aster_state;
pub mod credential_bridge;
pub mod event_converter;
pub mod subagent_scheduler;
pub mod types;

pub use aster_agent::{AsterAgentWrapper, SessionDetail, SessionInfo};
pub use aster_state::AsterAgentState;
pub use credential_bridge::{
    create_aster_provider, AsterProviderConfig, CredentialBridge, CredentialBridgeError,
};
pub use event_converter::{convert_agent_event, TauriAgentEvent};
pub use subagent_scheduler::{
    ProxyCastScheduler, ProxyCastSubAgentExecutor, SubAgentProgressEvent,
};
pub use types::*;
