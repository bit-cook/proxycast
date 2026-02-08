//! SubAgent 调度器集成
//!
//! 将 aster-rust 的 SubAgent 调度器与 ProxyCast 凭证池集成。
//! 纯逻辑位于此 crate，事件发送通过注入回调实现。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aster::agents::context::AgentContext;
use aster::agents::subagent_scheduler::{
    SchedulerConfig, SchedulerError, SchedulerExecutionResult, SchedulerProgress, SchedulerResult,
    SubAgentExecutor, SubAgentResult, SubAgentScheduler, SubAgentTask,
    TokenUsage as SchedulerTokenUsage,
};
use aster::conversation::message::Message;
use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::credential_bridge::{create_aster_provider, AsterProviderConfig, CredentialBridge};
use proxycast_core::database::DbConnection;

/// 调度器事件发射器
pub type SchedulerEventEmitter = Arc<dyn Fn(&serde_json::Value) + Send + Sync>;

/// ProxyCast SubAgent 执行器
///
/// 实现 aster-rust 的 SubAgentExecutor trait，
/// 集成 ProxyCast 凭证池进行 LLM 调用。
pub struct ProxyCastSubAgentExecutor {
    /// 凭证桥接器
    credential_bridge: CredentialBridge,
    /// 数据库连接
    db: DbConnection,
    /// 默认模型
    default_model: String,
    /// 默认 Provider 类型
    default_provider: String,
}

impl ProxyCastSubAgentExecutor {
    /// 创建新的执行器
    pub fn new(db: DbConnection) -> Self {
        Self {
            credential_bridge: CredentialBridge::new(),
            db,
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_provider: "anthropic".to_string(),
        }
    }

    /// 设置默认模型
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// 设置默认 Provider
    pub fn with_default_provider(mut self, provider: impl Into<String>) -> Self {
        self.default_provider = provider.into();
        self
    }

    /// 从凭证池选择凭证
    async fn select_credential(&self, task: &SubAgentTask) -> SchedulerResult<AsterProviderConfig> {
        let model = task.model.as_deref().unwrap_or(&self.default_model);
        let provider_type = &self.default_provider;

        let config = self
            .credential_bridge
            .select_and_configure(&self.db, provider_type, model)
            .await
            .map_err(|e| SchedulerError::ProviderError(e.to_string()))?;

        Ok(config)
    }

    /// 生成摘要
    fn generate_summary(&self, output: &str, task: &SubAgentTask) -> String {
        let max_len = 500;
        if output.chars().count() <= max_len {
            format!("任务 {} 完成:\n{}", task.id, output)
        } else {
            let truncated: String = output.chars().take(max_len - 3).collect();
            format!("任务 {} 完成:\n{}...", task.id, truncated)
        }
    }
}

#[async_trait::async_trait]
impl SubAgentExecutor for ProxyCastSubAgentExecutor {
    async fn execute_task(
        &self,
        task: &SubAgentTask,
        context: &AgentContext,
    ) -> SchedulerResult<SubAgentResult> {
        let start_time = Utc::now();
        info!("执行 SubAgent 任务: {}", task.id);

        let provider_config = self.select_credential(task).await?;
        debug!("使用凭证: {}", provider_config.credential_uuid);

        let provider = create_aster_provider(&provider_config)
            .await
            .map_err(|e| SchedulerError::ProviderError(e.to_string()))?;

        let system_prompt = context.system_prompt.clone().unwrap_or_default();
        let user_message = Message::user().with_text(&task.prompt);

        let (response_msg, usage) = provider
            .complete(&system_prompt, &[user_message], &[])
            .await
            .map_err(|e| SchedulerError::ProviderError(e.to_string()))?;

        let response = response_msg.as_concat_text();

        let end_time = Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or(Duration::ZERO);

        let summary = if task.return_summary {
            Some(self.generate_summary(&response, task))
        } else {
            None
        };

        let token_usage = Some(SchedulerTokenUsage {
            input_tokens: usage.usage.input_tokens.unwrap_or(0) as usize,
            output_tokens: usage.usage.output_tokens.unwrap_or(0) as usize,
            total_tokens: usage.usage.total_tokens.unwrap_or(0) as usize,
        });

        Ok(SubAgentResult {
            task_id: task.id.clone(),
            success: true,
            output: Some(response),
            summary,
            error: None,
            duration,
            retries: 0,
            started_at: start_time,
            completed_at: end_time,
            token_usage,
            metadata: HashMap::new(),
        })
    }
}

/// ProxyCast SubAgent 调度器
pub struct ProxyCastScheduler {
    /// 内部调度器
    scheduler: Arc<RwLock<Option<SubAgentScheduler<ProxyCastSubAgentExecutor>>>>,
    /// 数据库连接
    db: DbConnection,
}

impl ProxyCastScheduler {
    /// 创建新的调度器
    pub fn new(db: DbConnection) -> Self {
        Self {
            scheduler: Arc::new(RwLock::new(None)),
            db,
        }
    }

    /// 初始化调度器（不附带事件回调）
    pub async fn init(&self, config: Option<SchedulerConfig>) {
        self.init_with_event_emitter(config, None).await;
    }

    /// 初始化调度器（可附带事件回调）
    pub async fn init_with_event_emitter(
        &self,
        config: Option<SchedulerConfig>,
        event_emitter: Option<SchedulerEventEmitter>,
    ) {
        let executor = ProxyCastSubAgentExecutor::new(self.db.clone());
        let config = config.unwrap_or_default();

        let scheduler = if let Some(emitter) = event_emitter {
            SubAgentScheduler::new(config, executor).with_event_callback(move |event| {
                match serde_json::to_value(&event) {
                    Ok(payload) => emitter(&payload),
                    Err(err) => warn!("序列化调度事件失败: {}", err),
                }
            })
        } else {
            SubAgentScheduler::new(config, executor)
        };

        *self.scheduler.write().await = Some(scheduler);
        info!("ProxyCast SubAgent 调度器初始化完成");
    }

    /// 执行任务
    pub async fn execute(
        &self,
        tasks: Vec<SubAgentTask>,
        parent_context: Option<&AgentContext>,
    ) -> SchedulerResult<SchedulerExecutionResult> {
        let scheduler = self.scheduler.read().await;
        let scheduler = scheduler
            .as_ref()
            .ok_or_else(|| SchedulerError::ContextError("调度器未初始化".to_string()))?;

        scheduler.execute(tasks, parent_context).await
    }

    /// 取消执行
    pub async fn cancel(&self) {
        if let Some(scheduler) = self.scheduler.read().await.as_ref() {
            scheduler.cancel().await;
        }
    }
}

/// Tauri 事件：SubAgent 进度
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubAgentProgressEvent {
    /// 总任务数
    pub total: usize,
    /// 已完成数
    pub completed: usize,
    /// 失败数
    pub failed: usize,
    /// 运行中数
    pub running: usize,
    /// 进度百分比
    pub percentage: f64,
    /// 当前任务
    pub current_tasks: Vec<String>,
}

impl From<SchedulerProgress> for SubAgentProgressEvent {
    fn from(progress: SchedulerProgress) -> Self {
        Self {
            total: progress.total,
            completed: progress.completed,
            failed: progress.failed,
            running: progress.running,
            percentage: progress.percentage,
            current_tasks: progress.current_tasks,
        }
    }
}
