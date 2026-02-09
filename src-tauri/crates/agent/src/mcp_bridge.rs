//! MCP 桥接客户端
//!
//! 实现 Aster 的 McpClientTrait，将工具调用转发到
//! ProxyCast 已有的 MCP RunningService，避免重复启动进程。

use aster::agents::mcp_client::{Error as McpError, McpClientTrait};
use aster::session_context::{current_session_id, SESSION_ID_HEADER};
use proxycast_mcp::client::ProxyCastMcpClient;
use rmcp::model::{
    CallToolRequest, CallToolRequestParam, CallToolResult, CancelledNotification,
    CancelledNotificationMethod, CancelledNotificationParam, ClientRequest, GetPromptRequest,
    GetPromptRequestParam, GetPromptResult, InitializeResult, JsonObject, ListPromptsRequest,
    ListPromptsResult, ListResourcesRequest, ListResourcesResult, ListToolsRequest,
    ListToolsResult, Meta, PaginatedRequestParam, ReadResourceRequest, ReadResourceRequestParam,
    ReadResourceResult, ServerNotification, ServerResult,
};
use rmcp::service::{PeerRequestOptions, RunningService, ServiceError};
use rmcp::RoleClient;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// MCP 桥接客户端
///
/// 持有 ProxyCast 的 rmcp RunningService 引用，
/// 将 Aster 的工具调用转发到已有的 MCP 连接。
#[allow(dead_code)]
pub struct McpBridgeClient {
    /// 服务器名称
    name: String,
    /// ProxyCast 的 rmcp RunningService
    service: Arc<RunningService<RoleClient, ProxyCastMcpClient>>,
    /// ProxyCast MCP 客户端处理器
    handler: Arc<ProxyCastMcpClient>,
    /// 服务器初始化信息
    server_info: Option<InitializeResult>,
    /// 请求超时时间
    timeout: Duration,
}

impl McpBridgeClient {
    pub fn new(
        name: String,
        service: Arc<RunningService<RoleClient, ProxyCastMcpClient>>,
        handler: Arc<ProxyCastMcpClient>,
        server_info: Option<InitializeResult>,
    ) -> Self {
        Self {
            name,
            service,
            handler,
            server_info,
            timeout: Duration::from_secs(60), // 默认超时 60s
        }
    }

    /// 发送请求并处理取消和超时
    async fn send_request(
        &self,
        request: ClientRequest,
        cancel_token: CancellationToken,
    ) -> Result<ServerResult, McpError> {
        // 发送请求
        let handle = self
            .service
            .send_cancellable_request(request, PeerRequestOptions::no_options())
            .await?;

        let request_id = handle.id;
        let peer = handle.peer.clone();

        // 等待响应，同时处理超时和取消
        tokio::select! {
            result = handle.rx => {
                result.map_err(|_e| ServiceError::TransportClosed)?
            }
            _ = tokio::time::sleep(self.timeout) => {
                // 超时，发送取消通知
                let _ = peer.send_notification(
                    CancelledNotification {
                        params: CancelledNotificationParam {
                            request_id,
                            reason: Some("timed out".to_owned()),
                        },
                        method: CancelledNotificationMethod,
                        extensions: Default::default(),
                    }
                    .into(),
                ).await;
                Err(ServiceError::Timeout{timeout: self.timeout})
            }
            _ = cancel_token.cancelled() => {
                // 取消，发送取消通知
                let _ = peer.send_notification(
                    CancelledNotification {
                        params: CancelledNotificationParam {
                            request_id,
                            reason: Some("operation cancelled".to_owned()),
                        },
                        method: CancelledNotificationMethod,
                        extensions: Default::default(),
                    }
                    .into(),
                ).await;
                Err(ServiceError::Cancelled { reason: None })
            }
        }
    }

    /// 注入 Session ID 到扩展字段
    fn inject_session(&self, mut extensions: rmcp::model::Extensions) -> rmcp::model::Extensions {
        if let Some(session_id) = current_session_id() {
            let mut meta_map = extensions
                .get::<Meta>()
                .map(|meta| meta.0.clone())
                .unwrap_or_default();

            // 移除旧的 ID (大小写不敏感)
            meta_map.retain(|k, _| !k.eq_ignore_ascii_case(SESSION_ID_HEADER));
            // 插入新的 ID
            meta_map.insert(SESSION_ID_HEADER.to_string(), Value::String(session_id));

            extensions.insert(Meta(meta_map));
        }
        extensions
    }
}

#[async_trait::async_trait]
impl McpClientTrait for McpBridgeClient {
    async fn list_resources(
        &self,
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListResourcesResult, McpError> {
        let res = self
            .send_request(
                ClientRequest::ListResourcesRequest(ListResourcesRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: self.inject_session(Default::default()),
                }),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::ListResourcesResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn read_resource(
        &self,
        uri: &str,
        cancel_token: CancellationToken,
    ) -> Result<ReadResourceResult, McpError> {
        let res = self
            .send_request(
                ClientRequest::ReadResourceRequest(ReadResourceRequest {
                    params: ReadResourceRequestParam {
                        uri: uri.to_string(),
                    },
                    method: Default::default(),
                    extensions: self.inject_session(Default::default()),
                }),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::ReadResourceResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn list_tools(
        &self,
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, McpError> {
        let res = self
            .send_request(
                ClientRequest::ListToolsRequest(ListToolsRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: self.inject_session(Default::default()),
                }),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::ListToolsResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        let res = self
            .send_request(
                ClientRequest::CallToolRequest(CallToolRequest {
                    params: CallToolRequestParam {
                        name: name.to_string().into(),
                        arguments,
                    },
                    method: Default::default(),
                    extensions: self.inject_session(Default::default()),
                }),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::CallToolResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn list_prompts(
        &self,
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListPromptsResult, McpError> {
        let res = self
            .send_request(
                ClientRequest::ListPromptsRequest(ListPromptsRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: self.inject_session(Default::default()),
                }),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::ListPromptsResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Value,
        cancel_token: CancellationToken,
    ) -> Result<GetPromptResult, McpError> {
        let arguments = match arguments {
            Value::Object(map) => Some(map),
            _ => None,
        };
        let res = self
            .send_request(
                ClientRequest::GetPromptRequest(GetPromptRequest {
                    params: GetPromptRequestParam {
                        name: name.to_string(),
                        arguments,
                    },
                    method: Default::default(),
                    extensions: self.inject_session(Default::default()),
                }),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::GetPromptResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        self.handler.subscribe().await
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }
}
