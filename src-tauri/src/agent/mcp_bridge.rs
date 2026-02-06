//! MCP 桥接客户端
//!
//! 实现 Aster 的 McpClientTrait，将工具调用转发到
//! ProxyCast 已有的 MCP RunningService，避免重复启动进程。

use aster::agents::mcp_client::{Error, McpClientTrait};
use rmcp::model::{
    CallToolResult, GetPromptResult, InitializeResult, JsonObject,
    ListPromptsResult, ListResourcesResult, ListToolsResult,
    ReadResourceResult, ServerNotification,
};
use rmcp::service::RunningService;
use rmcp::RoleClient;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::mcp::client::ProxyCastMcpClient;

/// MCP 桥接客户端
///
/// 持有 ProxyCast 的 RunningService 引用，
/// 将 Aster 的工具调用转发到已有的 MCP 连接。
pub struct McpBridgeClient {
    /// 服务器名称
    name: String,
    /// ProxyCast 的 rmcp RunningService
    service: Arc<RunningService<RoleClient, ProxyCastMcpClient>>,
    /// 服务器初始化信息
    server_info: Option<InitializeResult>,
}

impl McpBridgeClient {
    pub fn new(
        name: String,
        service: Arc<RunningService<RoleClient, ProxyCastMcpClient>>,
        server_info: Option<InitializeResult>,
    ) -> Self {
        Self {
            name,
            service,
            server_info,
        }
    }
}
