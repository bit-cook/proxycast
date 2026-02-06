//! 错误类型模块
//!
//! 定义 ProxyCast 应用中的各种错误类型。
//!
//! ## 模块结构
//! - `project_error`: 项目相关错误（ProjectError, PersonaError, MaterialError, TemplateError, MigrationError）

pub mod project_error;

// 重新导出常用错误类型
#[allow(unused_imports)]
pub use project_error::{MaterialError, MigrationError, PersonaError, ProjectError, TemplateError};
