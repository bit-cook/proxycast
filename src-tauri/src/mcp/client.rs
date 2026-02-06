//! MCP 客户端实现
//!
//! 本模块实现 rmcp 的 ClientHandler trait，处理：
//! - 客户端信息返回
//! - 进度通知处理
//! - 日志消息处理
//! - 与 Tauri 事件系统的集成

#![allow(dead_code)]

use rmcp::{
    model::{
        ClientCapabilities, ClientInfo, Implementation, LoggingMessageNotification,
        LoggingMessageNotificationMethod, LoggingMessageNotificationParam, ProgressNotification,
        ProgressNotificationMethod, ProgressNotificationParam, ProtocolVersion, ServerNotification,
    },
    service::NotificationContext,
    ClientHandler, RoleClient,
};
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

/// 进度通知事件 Payload
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpProgressPayload {
    pub server_name: String,
    pub progress_token: String,
    pub progress: f64,
    pub total: Option<f64>,
    pub message: Option<String>,
}

/// 日志消息事件 Payload
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpLogMessagePayload {
    pub server_name: String,
    pub level: String,
    pub logger: Option<String>,
    pub data: serde_json::Value,
}

/// ProxyCast MCP 客户端处理器
///
/// 实现 rmcp::ClientHandler trait，处理 MCP 服务器的通知和回调
pub struct ProxyCastMcpClient {
    /// Tauri AppHandle（用于发送事件）
    app_handle: Option<tauri::AppHandle>,
    /// 服务器名称（用于事件标识）
    server_name: String,
    /// 通知订阅者（用于内部通知分发）
    notification_handlers: Arc<Mutex<Vec<mpsc::Sender<ServerNotification>>>>,
}

impl ProxyCastMcpClient {
    /// 创建新的 MCP 客户端处理器
    ///
    /// # Arguments
    /// * `server_name` - MCP 服务器名称，用于事件标识
    /// * `app_handle` - Tauri AppHandle，用于发送事件到前端
    pub fn new(server_name: String, app_handle: Option<tauri::AppHandle>) -> Self {
        Self {
            app_handle,
            server_name,
            notification_handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 获取通知处理器的引用（用于订阅通知）
    pub fn notification_handlers(&self) -> Arc<Mutex<Vec<mpsc::Sender<ServerNotification>>>> {
        self.notification_handlers.clone()
    }

    /// 订阅服务器通知
    ///
    /// 返回一个接收器，用于接收来自 MCP 服务器的通知
    pub async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        let (tx, rx) = mpsc::channel(16);
        self.notification_handlers.lock().await.push(tx);
        rx
    }

    /// 发送 Tauri 事件到前端
    fn emit_event<T: serde::Serialize + Clone>(&self, event: &str, payload: T) {
        if let Some(ref app_handle) = self.app_handle {
            if let Err(e) = app_handle.emit(event, payload) {
                warn!(
                    server_name = %self.server_name,
                    event = %event,
                    error = %e,
                    "发送 Tauri 事件失败"
                );
            }
        }
    }
}

impl ClientHandler for ProxyCastMcpClient {
    /// 返回客户端信息
    ///
    /// 提供 ProxyCast 客户端的标识信息，包括：
    /// - 协议版本
    /// - 客户端能力（采样支持）
    /// - 客户端实现信息
    fn get_info(&self) -> ClientInfo {
        ClientInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ClientCapabilities::builder().enable_sampling().build(),
            client_info: Implementation {
                name: "proxycast".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                title: Some("ProxyCast MCP Client".to_string()),
                website_url: Some("https://github.com/aiclientproxy/proxycast".to_string()),
            },
        }
    }

    /// 处理进度通知
    ///
    /// 当 MCP 服务器发送进度更新时调用此方法。
    /// 进度信息会：
    /// 1. 记录到日志
    /// 2. 发送到 Tauri 事件系统（前端可监听）
    /// 3. 分发给内部通知订阅者
    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        context: NotificationContext<RoleClient>,
    ) {
        // 记录进度日志
        debug!(
            server_name = %self.server_name,
            progress_token = ?params.progress_token,
            progress = params.progress,
            total = ?params.total,
            "收到 MCP 进度通知"
        );

        // 发送 Tauri 事件到前端
        let payload = McpProgressPayload {
            server_name: self.server_name.clone(),
            progress_token: format!("{:?}", params.progress_token),
            progress: params.progress,
            total: params.total,
            message: None,
        };
        self.emit_event("mcp:progress", payload);

        // 分发给内部通知订阅者
        let notification = ServerNotification::ProgressNotification(ProgressNotification {
            params: params.clone(),
            method: ProgressNotificationMethod,
            extensions: context.extensions.clone(),
        });

        let handlers = self.notification_handlers.lock().await;
        for handler in handlers.iter() {
            let _ = handler.try_send(notification.clone());
        }
    }

    /// 处理日志消息通知
    ///
    /// 当 MCP 服务器发送日志消息时调用此方法。
    /// 日志消息会：
    /// 1. 根据级别记录到本地日志
    /// 2. 发送到 Tauri 事件系统（前端可监听）
    /// 3. 分发给内部通知订阅者
    async fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
        context: NotificationContext<RoleClient>,
    ) {
        // 根据日志级别记录
        let level_str = format!("{:?}", params.level);
        match params.level {
            rmcp::model::LoggingLevel::Debug => {
                debug!(
                    server_name = %self.server_name,
                    logger = ?params.logger,
                    data = ?params.data,
                    "MCP 服务器日志 [DEBUG]"
                );
            }
            rmcp::model::LoggingLevel::Info => {
                info!(
                    server_name = %self.server_name,
                    logger = ?params.logger,
                    data = ?params.data,
                    "MCP 服务器日志 [INFO]"
                );
            }
            rmcp::model::LoggingLevel::Notice => {
                info!(
                    server_name = %self.server_name,
                    logger = ?params.logger,
                    data = ?params.data,
                    "MCP 服务器日志 [NOTICE]"
                );
            }
            rmcp::model::LoggingLevel::Warning => {
                warn!(
                    server_name = %self.server_name,
                    logger = ?params.logger,
                    data = ?params.data,
                    "MCP 服务器日志 [WARNING]"
                );
            }
            rmcp::model::LoggingLevel::Error => {
                tracing::error!(
                    server_name = %self.server_name,
                    logger = ?params.logger,
                    data = ?params.data,
                    "MCP 服务器日志 [ERROR]"
                );
            }
            rmcp::model::LoggingLevel::Critical => {
                tracing::error!(
                    server_name = %self.server_name,
                    logger = ?params.logger,
                    data = ?params.data,
                    "MCP 服务器日志 [CRITICAL]"
                );
            }
            rmcp::model::LoggingLevel::Alert => {
                tracing::error!(
                    server_name = %self.server_name,
                    logger = ?params.logger,
                    data = ?params.data,
                    "MCP 服务器日志 [ALERT]"
                );
            }
            rmcp::model::LoggingLevel::Emergency => {
                tracing::error!(
                    server_name = %self.server_name,
                    logger = ?params.logger,
                    data = ?params.data,
                    "MCP 服务器日志 [EMERGENCY]"
                );
            }
        }

        // 发送 Tauri 事件到前端
        let payload = McpLogMessagePayload {
            server_name: self.server_name.clone(),
            level: level_str,
            logger: params.logger.clone(),
            data: params.data.clone(),
        };
        self.emit_event("mcp:log_message", payload);

        // 分发给内部通知订阅者
        let notification =
            ServerNotification::LoggingMessageNotification(LoggingMessageNotification {
                params: params.clone(),
                method: LoggingMessageNotificationMethod,
                extensions: context.extensions.clone(),
            });

        let handlers = self.notification_handlers.lock().await;
        for handler in handlers.iter() {
            let _ = handler.try_send(notification.clone());
        }
    }
}

/// MCP 客户端包装器
///
/// 封装 rmcp 客户端和相关状态
pub struct McpClientWrapper {
    /// 服务器名称
    pub server_name: String,
    /// 服务器配置
    pub config: super::types::McpServerConfig,
    /// 子进程句柄
    pub process: Option<tokio::process::Child>,
    /// 服务器能力信息
    pub server_info: Option<super::types::McpServerCapabilities>,
    /// 客户端处理器
    pub client_handler: Arc<ProxyCastMcpClient>,
    /// rmcp 运行服务（用于发送请求）
    pub running_service:
        Option<rmcp::service::RunningService<rmcp::RoleClient, ProxyCastMcpClient>>,
}

impl McpClientWrapper {
    /// 创建新的客户端包装器
    pub fn new(
        server_name: String,
        config: super::types::McpServerConfig,
        app_handle: Option<tauri::AppHandle>,
    ) -> Self {
        let client_handler = Arc::new(ProxyCastMcpClient::new(server_name.clone(), app_handle));

        Self {
            server_name,
            config,
            process: None,
            server_info: None,
            client_handler,
            running_service: None,
        }
    }

    /// 获取客户端处理器的引用
    pub fn handler(&self) -> Arc<ProxyCastMcpClient> {
        self.client_handler.clone()
    }

    /// 设置子进程句柄
    pub fn set_process(&mut self, process: tokio::process::Child) {
        self.process = Some(process);
    }

    /// 设置服务器能力信息
    pub fn set_server_info(&mut self, info: super::types::McpServerCapabilities) {
        self.server_info = Some(info);
    }

    /// 设置 rmcp 运行服务
    pub fn set_running_service(
        &mut self,
        service: rmcp::service::RunningService<rmcp::RoleClient, ProxyCastMcpClient>,
    ) {
        self.running_service = Some(service);
    }

    /// 获取 rmcp 运行服务的引用
    pub fn running_service(
        &self,
    ) -> Option<&rmcp::service::RunningService<rmcp::RoleClient, ProxyCastMcpClient>> {
        self.running_service.as_ref()
    }

    /// 终止子进程
    pub async fn kill_process(&mut self) -> Result<(), std::io::Error> {
        if let Some(ref mut process) = self.process {
            process.kill().await?;
        }
        self.process = None;
        self.running_service = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_info() {
        let client = ProxyCastMcpClient::new("test-server".to_string(), None);
        let info = client.get_info();

        assert_eq!(info.client_info.name, "proxycast");
        assert_eq!(info.client_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            info.client_info.title,
            Some("ProxyCast MCP Client".to_string())
        );
        assert_eq!(info.protocol_version, ProtocolVersion::V_2025_03_26);
    }

    #[test]
    fn test_client_wrapper_creation() {
        let config = super::super::types::McpServerConfig {
            command: "test-command".to_string(),
            args: vec!["--arg1".to_string()],
            env: std::collections::HashMap::new(),
            cwd: None,
            timeout: 30,
        };

        let wrapper = McpClientWrapper::new("test-server".to_string(), config.clone(), None);

        assert_eq!(wrapper.server_name, "test-server");
        assert_eq!(wrapper.config.command, "test-command");
        assert!(wrapper.process.is_none());
        assert!(wrapper.server_info.is_none());
    }

    #[tokio::test]
    async fn test_notification_subscription() {
        let client = ProxyCastMcpClient::new("test-server".to_string(), None);

        // 订阅通知
        let mut rx = client.subscribe().await;

        // 验证订阅者已添加
        let handlers = client.notification_handlers.lock().await;
        assert_eq!(handlers.len(), 1);
        drop(handlers);

        // 验证接收器可用（不会阻塞）
        assert!(rx.try_recv().is_err()); // 应该是空的
    }
}
