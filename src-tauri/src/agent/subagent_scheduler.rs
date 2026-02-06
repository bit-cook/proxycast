//! SubAgent 调度器集成
//!
//! 将 aster-rust 的 SubAgent 调度器与 ProxyCast 凭证池集成
//!
//! ## 功能
//! - 自动从凭证池选择健康凭证
//! - 支持凭证 fallback 策略
//! - 集成 Tauri 事件系统进行进度通知

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aster::agents::context::AgentContext;
use aster::agents::subagent_scheduler::{
    SchedulerConfig, SchedulerError, SchedulerExecutionResult, SchedulerResult, SubAgentExecutor,
    SubAgentResult, SubAgentScheduler, SubAgentTask, TokenUsage as SchedulerTokenUsage,
};
use aster::conversation::message::Message;
use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::agent::credential_bridge::{
    create_aster_provider, AsterProviderConfig, CredentialBridge,
};
use crate::database::DbConnection;

/// ProxyCast SubAgent 执行器
///
/// 实现 aster-rust 的 SubAgentExecutor trait，
/// 集成 ProxyCast 凭证池进行 LLM 调用
pub struct ProxyCastSubAgentExecutor {
    /// 凭证桥接器
    credential_bridge: CredentialBridge,
    /// 数据库连接
    db: DbConnection,
    /// 默认模型
    default_model: String,
    /// 默认 Provider 类型
    default_provider: String,
    /// Tauri AppHandle（用于事件通知）
    app_handle: Option<AppHandle>,
}

impl ProxyCastSubAgentExecutor {
    /// 创建新的执行器
    pub fn new(db: DbConnection) -> Self {
        Self {
            credential_bridge: CredentialBridge::new(),
            db,
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_provider: "anthropic".to_string(),
            app_handle: None,
        }
    }

    /// 设置 Tauri AppHandle
    pub fn with_app_handle(mut self, handle: AppHandle) -> Self {
        self.app_handle = Some(handle);
        self
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
        // 根据任务类型和模型选择 provider
        let model = task.model.as_deref().unwrap_or(&self.default_model);
        let provider_type = &self.default_provider;

        // 使用 CredentialBridge 选择凭证
        let config = self
            .credential_bridge
            .select_and_configure(&self.db, provider_type, model)
            .await
            .map_err(|e| SchedulerError::ProviderError(e.to_string()))?;

        Ok(config)
    }

    /// 发送 Tauri 事件
    #[allow(dead_code)]
    fn emit_event(&self, event_name: &str, payload: impl serde::Serialize + Clone) {
        if let Some(handle) = &self.app_handle {
            if let Err(e) = handle.emit(event_name, payload) {
                warn!("发送 Tauri 事件失败: {}", e);
            }
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

        // 选择凭证
        let provider_config = self.select_credential(task).await?;
        debug!("使用凭证: {}", provider_config.credential_uuid);

        // 创建 provider
        let provider = create_aster_provider(&provider_config)
            .await
            .map_err(|e| SchedulerError::ProviderError(e.to_string()))?;

        // 构建提示
        let system_prompt = context.system_prompt.clone().unwrap_or_default();
        let user_message = Message::user().with_text(&task.prompt);

        // 调用 LLM（使用 complete 方法）
        let (response_msg, usage) = provider
            .complete(&system_prompt, &[user_message], &[])
            .await
            .map_err(|e| SchedulerError::ProviderError(e.to_string()))?;

        let response = response_msg.as_concat_text();

        let end_time = Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or(Duration::ZERO);

        // 生成摘要
        let summary = if task.return_summary {
            Some(self.generate_summary(&response, task))
        } else {
            None
        };

        // 转换 token 使用
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

impl ProxyCastSubAgentExecutor {
    /// 生成摘要
    fn generate_summary(&self, output: &str, task: &SubAgentTask) -> String {
        // 简单摘要：取前 500 字符
        let max_len = 500;
        if output.chars().count() <= max_len {
            format!("任务 {} 完成:\n{}", task.id, output)
        } else {
            let truncated: String = output.chars().take(max_len - 3).collect();
            format!("任务 {} 完成:\n{}...", task.id, truncated)
        }
    }
}

/// ProxyCast SubAgent 调度器包装器
pub struct ProxyCastScheduler {
    /// 内部调度器
    scheduler: Arc<RwLock<Option<SubAgentScheduler<ProxyCastSubAgentExecutor>>>>,
    /// 数据库连接
    db: DbConnection,
    /// Tauri AppHandle
    app_handle: Option<AppHandle>,
}

impl ProxyCastScheduler {
    /// 创建新的调度器
    pub fn new(db: DbConnection) -> Self {
        Self {
            scheduler: Arc::new(RwLock::new(None)),
            db,
            app_handle: None,
        }
    }

    /// 设置 Tauri AppHandle
    pub fn with_app_handle(mut self, handle: AppHandle) -> Self {
        self.app_handle = Some(handle);
        self
    }

    /// 初始化调度器
    pub async fn init(&self, config: Option<SchedulerConfig>) {
        let executor = ProxyCastSubAgentExecutor::new(self.db.clone());
        let executor = if let Some(handle) = &self.app_handle {
            executor.with_app_handle(handle.clone())
        } else {
            executor
        };

        let config = config.unwrap_or_default();

        // 创建调度器并设置事件回调
        let app_handle = self.app_handle.clone();
        let scheduler =
            SubAgentScheduler::new(config, executor).with_event_callback(move |event| {
                if let Some(handle) = &app_handle {
                    let _ = handle.emit("subagent-scheduler-event", &event);
                }
            });

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

impl From<aster::agents::subagent_scheduler::SchedulerProgress> for SubAgentProgressEvent {
    fn from(p: aster::agents::subagent_scheduler::SchedulerProgress) -> Self {
        Self {
            total: p.total,
            completed: p.completed,
            failed: p.failed,
            running: p.running,
            percentage: p.percentage,
            current_tasks: p.current_tasks,
        }
    }
}
