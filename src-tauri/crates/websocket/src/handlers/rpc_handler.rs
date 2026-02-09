//! RPC 处理器
//!
//! 处理 Gateway RPC 请求，集成 Agent 和 Scheduler

use super::super::{protocol::*, WsError};
use std::sync::Arc;
use tokio::sync::RwLock;

/// RPC 处理器状态
#[derive(Clone)]
pub struct RpcHandlerState {
    /// 数据库连接
    pub db: Arc<RwLock<Option<proxycast_core::database::DbConnection>>>,
    /// Agent 调度器（可选）
    pub scheduler: Arc<RwLock<Option<proxycast_agent::ProxyCastScheduler>>>,
    /// 日志存储
    pub logs: Arc<RwLock<proxycast_core::LogStore>>,
}

impl RpcHandlerState {
    /// 创建新的 RPC 处理器状态
    pub fn new(
        db: Option<proxycast_core::database::DbConnection>,
        scheduler: Option<proxycast_agent::ProxyCastScheduler>,
        logs: Arc<RwLock<proxycast_core::LogStore>>,
    ) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            scheduler: Arc::new(RwLock::new(scheduler)),
            logs,
        }
    }
}

/// RPC 处理器
pub struct RpcHandler {
    state: RpcHandlerState,
}

impl RpcHandler {
    /// 创建新的 RPC 处理器
    pub fn new(state: RpcHandlerState) -> Self {
        Self { state }
    }

    /// 处理 RPC 请求
    pub async fn handle_request(&self, request: GatewayRpcRequest) -> GatewayRpcResponse {
        let method = request.method;
        let request_id = request.id.clone();
        let params = request.params;

        // 记录请求
        self.state.logs.write().await.add(
            "info",
            &format!("[RPC] Request: id={} method={:?}", request_id, method),
        );

        // 路由到具体的处理方法
        let result = match method {
            RpcMethod::AgentRun => self.handle_agent_run(params).await,
            RpcMethod::AgentWait => self.handle_agent_wait(params).await,
            RpcMethod::AgentStop => self.handle_agent_stop(params).await,
            RpcMethod::SessionsList => self.handle_sessions_list().await,
            RpcMethod::SessionsGet => self.handle_sessions_get(params).await,
            RpcMethod::CronList => self.handle_cron_list().await,
            RpcMethod::CronRun => self.handle_cron_run(params).await,
        };

        match result {
            Ok(data) => GatewayRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request_id,
                result: Some(data),
                error: None,
            },
            Err(err) => {
                self.state.logs.write().await.add(
                    "error",
                    &format!("[RPC] Error: id={} error={}", request_id, err.message),
                );
                GatewayRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    result: None,
                    error: Some(err),
                }
            }
        }
    }

    /// 处理 agent.run
    async fn handle_agent_run(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, RpcError> {
        let params: AgentRunParams = params
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| {
                RpcError::invalid_params("Missing or invalid parameters for agent.run")
            })?;

        // TODO: 实现 Agent 运行逻辑
        // 1. 获取或创建会话
        // 2. 发送消息到 Agent
        // 3. 返回运行 ID

        let run_id = uuid::Uuid::new_v4().to_string();
        let session_id = params
            .session_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let result = AgentRunResult {
            run_id: run_id.clone(),
            session_id,
            completed: false, // 流式模式下未完成
            content: None,
            usage: None,
        };

        Ok(serde_json::to_value(result).map_err(|e| RpcError::internal_error(e.to_string()))?)
    }

    /// 处理 agent.wait
    async fn handle_agent_wait(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, RpcError> {
        let params: AgentWaitParams = params
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| {
                RpcError::invalid_params("Missing or invalid parameters for agent.wait")
            })?;

        // TODO: 实现 Agent 等待逻辑
        // 1. 等待运行完成
        // 2. 获取结果

        let result = AgentWaitResult {
            run_id: params.run_id,
            completed: true,
            content: Some("Response content".to_string()),
            usage: Some(TokenUsage::new(100, 200)),
        };

        Ok(serde_json::to_value(result).map_err(|e| RpcError::internal_error(e.to_string()))?)
    }

    /// 处理 agent.stop
    async fn handle_agent_stop(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, RpcError> {
        let params: AgentStopParams = params
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| {
                RpcError::invalid_params("Missing or invalid parameters for agent.stop")
            })?;

        // TODO: 实现 Agent 停止逻辑
        // 1. 取消运行
        // 2. 清理资源

        let result = AgentStopResult {
            run_id: params.run_id,
            stopped: true,
        };

        Ok(serde_json::to_value(result).map_err(|e| RpcError::internal_error(e.to_string()))?)
    }

    /// 处理 sessions.list
    async fn handle_sessions_list(&self) -> Result<serde_json::Value, RpcError> {
        // TODO: 从数据库获取会话列表
        let sessions = vec![];

        let result = SessionsListResult { sessions };

        Ok(serde_json::to_value(result).map_err(|e| RpcError::internal_error(e.to_string()))?)
    }

    /// 处理 sessions.get
    async fn handle_sessions_get(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, RpcError> {
        let params: SessionGetParams = params
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| {
                RpcError::invalid_params("Missing or invalid parameters for sessions.get")
            })?;

        // TODO: 从数据库获取会话详情
        let result = SessionGetResult {
            session_id: params.session_id,
            model: "claude-sonnet-4-5".to_string(),
            system_prompt: None,
            message_count: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(serde_json::to_value(result).map_err(|e| RpcError::internal_error(e.to_string()))?)
    }

    /// 处理 cron.list
    async fn handle_cron_list(&self) -> Result<serde_json::Value, RpcError> {
        // TODO: 从数据库获取定时任务列表
        let tasks = vec![];

        let result = CronListResult { tasks };

        Ok(serde_json::to_value(result).map_err(|e| RpcError::internal_error(e.to_string()))?)
    }

    /// 处理 cron.run
    async fn handle_cron_run(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, RpcError> {
        let params: CronRunParams = params
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| {
                RpcError::invalid_params("Missing or invalid parameters for cron.run")
            })?;

        // TODO: 实现定时任务运行逻辑
        // 1. 查找任务
        // 2. 提交到调度器
        // 3. 返回执行 ID

        let execution_id = uuid::Uuid::new_v4().to_string();

        let result = CronRunResult {
            task_id: params.task_id,
            execution_id,
            started: true,
        };

        Ok(serde_json::to_value(result).map_err(|e| RpcError::internal_error(e.to_string()))?)
    }
}

/// 从 WsMessage 解析 RPC 请求
pub fn parse_rpc_request(msg: &str) -> Result<GatewayRpcRequest, RpcError> {
    serde_json::from_str(msg).map_err(|e| RpcError::parse_error(format!("Invalid JSON: {}", e)))
}

/// 序列化 RPC 响应
pub fn serialize_rpc_response(resp: &GatewayRpcResponse) -> Result<String, WsError> {
    serde_json::to_string(resp)
        .map_err(|e| WsError::internal(None, format!("Failed to serialize response: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rpc_request() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": "test-123",
            "method": "agent.run",
            "params": {
                "message": "Hello",
                "stream": false
            }
        }"#;

        let request = parse_rpc_request(json).unwrap();
        assert_eq!(request.method, RpcMethod::AgentRun);
        assert_eq!(request.id, "test-123");
    }

    #[test]
    fn test_serialize_rpc_response() {
        let response = GatewayRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: "test-123".to_string(),
            result: Some(serde_json::json!({"success": true})),
            error: None,
        };

        let json = serialize_rpc_response(&response).unwrap();
        assert!(json.contains("2.0"));
        assert!(json.contains("test-123"));
    }
}
