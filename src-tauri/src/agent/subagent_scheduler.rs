//! SubAgent 调度器集成（Tauri 桥接层）
//!
//! 纯逻辑已迁移到 `proxycast-agent` crate，
//! 本模块负责 Tauri 事件桥接。

use std::sync::Arc;

use aster::agents::context::AgentContext;
use aster::agents::subagent_scheduler::{
    SchedulerConfig, SchedulerExecutionResult, SchedulerResult, SubAgentTask,
};
use tauri::{AppHandle, Emitter};

use crate::database::DbConnection;

pub use proxycast_agent::subagent_scheduler::{
    ProxyCastSubAgentExecutor, SchedulerEventEmitter, SubAgentProgressEvent,
};

/// ProxyCast SubAgent 调度器（Tauri 桥接）
pub struct ProxyCastScheduler {
    /// 内部纯逻辑调度器
    inner: proxycast_agent::subagent_scheduler::ProxyCastScheduler,
    /// Tauri AppHandle
    app_handle: Option<AppHandle>,
}

impl ProxyCastScheduler {
    /// 创建新的调度器
    pub fn new(db: DbConnection) -> Self {
        Self {
            inner: proxycast_agent::subagent_scheduler::ProxyCastScheduler::new(db),
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
        let event_emitter = self.app_handle.clone().map(|handle| {
            Arc::new(move |event: &serde_json::Value| {
                if let Err(err) = handle.emit("subagent-scheduler-event", event) {
                    tracing::warn!("发送 Tauri 事件失败: {}", err);
                }
            }) as SchedulerEventEmitter
        });

        self.inner
            .init_with_event_emitter(config, event_emitter)
            .await;
    }

    /// 执行任务
    pub async fn execute(
        &self,
        tasks: Vec<SubAgentTask>,
        parent_context: Option<&AgentContext>,
    ) -> SchedulerResult<SchedulerExecutionResult> {
        self.inner.execute(tasks, parent_context).await
    }

    /// 取消执行
    pub async fn cancel(&self) {
        self.inner.cancel().await;
    }
}
