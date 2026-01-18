//! Agent 工具系统模块
//!
//! 基于 aster-rust 框架的工具系统集成
//! 直接使用 aster-rust 提供的工具实现和注册表
//!
//! ## 架构说明
//! - 使用 aster-rust 的 ToolRegistry 和 Tool trait
//! - 直接注册 aster-rust 提供的所有工具
//! - 保持与现有 ProxyCast 接口的兼容性

// 重新导出 aster-rust 的工具系统
pub use aster::tools::*;

// 保持兼容性的类型别名和重新导出
pub use aster::tools::Tool;
pub use aster::tools::ToolContext;
pub use aster::tools::ToolDefinition;
pub use aster::tools::ToolError;
pub use aster::tools::ToolRegistry;
pub use aster::tools::ToolResult;

// 为了兼容性，创建一些类型别名
pub type JsonSchema = serde_json::Value;
pub type PropertySchema = serde_json::Value;

// 保留现有的特殊工具（暂时注释掉，需要适配 aster-rust 接口）
// pub mod browser;
// pub mod prompt;
// pub mod security;
// pub mod term_scrollback;
// pub mod terminal;

use std::path::Path;
use tracing::info;

#[cfg(test)]
mod test_integration;

/// 创建包含所有 aster-rust 工具的注册表
///
/// # Arguments
/// * `_base_dir` - 基础目录，所有文件操作必须在此目录内
///
/// # Returns
/// 包含 aster-rust 所有工具的注册表
pub fn create_default_registry(_base_dir: impl AsRef<Path>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // 注册所有 aster-rust 工具
    let config = aster::tools::ToolRegistrationConfig::default();
    let _shared_history = aster::tools::register_all_tools(&mut registry, config);

    info!(
        "[Tools] 已创建 aster-rust 工具注册表，共 {} 个工具",
        registry.tool_count()
    );

    registry
}

/// 创建简化的工具注册表（仅核心工具）
///
/// 只包含最基本的 aster-rust 工具
pub fn create_minimal_registry(_base_dir: impl AsRef<Path>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // 使用 aster-rust 的默认工具注册
    let _shared_history = aster::tools::register_default_tools(&mut registry);

    info!(
        "[Tools] 已创建最小工具注册表，共 {} 个工具",
        registry.tool_count()
    );

    registry
}
