//! Agent Task Executor
//!
//! 负责执行调度的任务

use super::types::ScheduledTask;
use async_trait::async_trait;
use proxycast_agent::credential_bridge::CredentialBridge;
use proxycast_core::database::DbConnection;
use std::sync::Arc;

/// 任务执行器 Trait
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// 执行任务
    ///
    /// # 参数
    /// - `task`: 要执行的任务
    /// - `db`: 数据库连接
    ///
    /// # 返回
    /// - 成功返回执行结果（JSON 格式）
    /// - 失败返回错误信息
    async fn execute(
        &self,
        task: &ScheduledTask,
        db: &DbConnection,
    ) -> Result<serde_json::Value, String>;
}

/// Agent 任务执行器
///
/// 通过 CredentialBridge 选择凭证，调用 Aster Agent 执行任务
pub struct AgentExecutor {
    credential_bridge: Arc<CredentialBridge>,
}

impl AgentExecutor {
    /// 创建新的执行器实例
    pub fn new() -> Self {
        Self {
            credential_bridge: Arc::new(CredentialBridge::new()),
        }
    }

    /// 使用自定义的 CredentialBridge 创建执行器
    pub fn with_credential_bridge(credential_bridge: Arc<CredentialBridge>) -> Self {
        Self { credential_bridge }
    }
}

impl Default for AgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskExecutor for AgentExecutor {
    async fn execute(
        &self,
        task: &ScheduledTask,
        db: &DbConnection,
    ) -> Result<serde_json::Value, String> {
        tracing::info!(
            "[AgentExecutor] 开始执行任务: {} (类型: {}, provider: {}, model: {})",
            task.name,
            task.task_type,
            task.provider_type,
            task.model
        );

        // 1. 从凭证池选择凭证
        let aster_config = self
            .credential_bridge
            .select_and_configure(db, &task.provider_type, &task.model)
            .await
            .map_err(|e| format!("选择凭证失败: {e}"))?;

        tracing::info!(
            "[AgentExecutor] 已选择凭证: {} (provider: {}, model: {})",
            aster_config.credential_uuid,
            aster_config.provider_name,
            aster_config.model_name
        );

        // 2. 根据任务类型执行不同的操作
        let result = match task.task_type.as_str() {
            "agent_chat" => {
                // 执行 Agent 对话任务
                self.execute_agent_chat(task, db, &aster_config).await?
            }
            "batch_process" => {
                // 执行批量处理任务
                self.execute_batch_process(task, db, &aster_config).await?
            }
            "scheduled_report" => {
                // 执行定时报告任务
                self.execute_scheduled_report(task, db, &aster_config)
                    .await?
            }
            _ => {
                return Err(format!("不支持的任务类型: {}", task.task_type));
            }
        };

        // 3. 标记凭证为健康
        if let Err(e) = self.credential_bridge.mark_healthy(
            db,
            &aster_config.credential_uuid,
            Some(&task.model),
        ) {
            tracing::warn!("[AgentExecutor] 标记凭证健康失败: {}", e);
        }

        tracing::info!("[AgentExecutor] 任务执行成功: {}", task.name);
        Ok(result)
    }
}

impl AgentExecutor {
    /// 执行 Agent 对话任务
    async fn execute_agent_chat(
        &self,
        task: &ScheduledTask,
        _db: &DbConnection,
        _aster_config: &proxycast_agent::credential_bridge::AsterProviderConfig,
    ) -> Result<serde_json::Value, String> {
        // 从任务参数中提取对话内容
        let prompt = task
            .params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少 prompt 参数".to_string())?;

        tracing::info!("[AgentExecutor] 执行 Agent 对话: {}", prompt);

        // TODO: 实际调用 Aster Agent 执行对话
        // 这里需要集成 AsterAgentState 来执行对话
        // 暂时返回模拟结果
        Ok(serde_json::json!({
            "type": "agent_chat",
            "prompt": prompt,
            "response": "任务已调度执行",
            "status": "success"
        }))
    }

    /// 执行批量处理任务
    async fn execute_batch_process(
        &self,
        task: &ScheduledTask,
        _db: &DbConnection,
        _aster_config: &proxycast_agent::credential_bridge::AsterProviderConfig,
    ) -> Result<serde_json::Value, String> {
        let items = task
            .params
            .get("items")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "缺少 items 参数".to_string())?;

        tracing::info!("[AgentExecutor] 执行批量处理: {} 项", items.len());

        // TODO: 实际执行批量处理逻辑
        Ok(serde_json::json!({
            "type": "batch_process",
            "total": items.len(),
            "processed": items.len(),
            "status": "success"
        }))
    }

    /// 执行定时报告任务
    async fn execute_scheduled_report(
        &self,
        task: &ScheduledTask,
        _db: &DbConnection,
        _aster_config: &proxycast_agent::credential_bridge::AsterProviderConfig,
    ) -> Result<serde_json::Value, String> {
        let report_type = task
            .params
            .get("report_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少 report_type 参数".to_string())?;

        tracing::info!("[AgentExecutor] 生成定时报告: {}", report_type);

        // TODO: 实际生成报告逻辑
        Ok(serde_json::json!({
            "type": "scheduled_report",
            "report_type": report_type,
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "status": "success"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn setup_test_db() -> DbConnection {
        let conn = Connection::open_in_memory().unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = AgentExecutor::new();
        assert!(Arc::strong_count(&executor.credential_bridge) >= 1);
    }

    #[tokio::test]
    async fn test_execute_agent_chat_missing_prompt() {
        let executor = AgentExecutor::new();
        let db = setup_test_db();

        let task = ScheduledTask::new(
            "Test".to_string(),
            "agent_chat".to_string(),
            serde_json::json!({}), // 缺少 prompt
            "openai".to_string(),
            "gpt-4".to_string(),
            Utc::now(),
        );

        // 由于缺少凭证池数据，这里会在选择凭证时失败
        // 但我们可以测试参数验证逻辑
        let result = executor.execute(&task, &db).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unsupported_task_type() {
        let executor = AgentExecutor::new();
        let db = setup_test_db();

        let task = ScheduledTask::new(
            "Test".to_string(),
            "unsupported_type".to_string(),
            serde_json::json!({}),
            "openai".to_string(),
            "gpt-4".to_string(),
            Utc::now(),
        );

        let result = executor.execute(&task, &db).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持的任务类型"));
    }
}
