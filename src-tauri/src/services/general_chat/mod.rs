//! 通用对话服务模块
//!
//! 提供通用对话功能的核心后端服务，包括：
//! - 会话管理（创建、删除、重命名）
//! - 消息存储和检索
//! - 会话标题自动生成
//!
//! ## 模块结构
//! - `types` - 核心数据类型定义
//! - `session_service` - 会话管理服务

pub mod session_service;
pub mod types;

pub use session_service::SessionService;
pub use types::*;
