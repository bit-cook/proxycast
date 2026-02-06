//! MCP（Model Context Protocol）模块
//!
//! 本模块提供 MCP 协议的客户端实现，支持：
//! - MCP 服务器生命周期管理（启动、停止、状态监控）
//! - MCP 工具发现和调用
//! - MCP 提示词和资源访问
//! - 工具格式转换（OpenAI/Anthropic/Gemini）
//!
//! # 模块结构
//!
//! - `types`: MCP 数据类型定义
//! - `client`: MCP 客户端实现（rmcp ClientHandler）
//! - `manager`: MCP 客户端管理器（连接池、缓存）
//! - `tool_converter`: 工具格式转换器

pub mod client;
pub mod manager;
pub mod tool_converter;
pub mod types;

// 显式导出，避免命名冲突
pub use client::{McpClientWrapper, ProxyCastMcpClient};
pub use manager::McpClientManager;
pub use tool_converter::ToolConverter;
pub use types::{
    McpContent, McpError, McpManagerState, McpPromptArgument, McpPromptDefinition,
    McpPromptMessage, McpPromptResult, McpResourceContent, McpResourceDefinition,
    McpServerCapabilities, McpServerConfig, McpServerErrorPayload, McpServerInfo,
    McpServerStartedPayload, McpServerStoppedPayload, McpToolCall, McpToolDefinition,
    McpToolResult, McpToolsUpdatedPayload,
};
