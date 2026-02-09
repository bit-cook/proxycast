//! Agent Scheduler 服务
//!
//! 提供后台心跳循环，定期检查并执行到期任务

use proxycast_core::database::DbConnection;
use proxycast_scheduler::{AgentExecutor, AgentScheduler, SchedulerTrait, TaskExecutor};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

/// 调度器服务配置
#[derive(Debug, Clone)]
pub struct SchedulerServiceConfig {
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
    /// 每次轮询的最大任务数
    pub max_tasks_per_poll: usize,
    /// 是否启用调度器
    pub enabled: bool,
}

impl Default for SchedulerServiceConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_secs: 30,
            max_tasks_per_poll: 10,
            enabled: true,
        }
    }
}

/// 调度器服务
///
/// 负责启动和管理后台心跳循环
pub struct SchedulerService {
    scheduler: Arc<AgentScheduler>,
    executor: Arc<AgentExecutor>,
    config: SchedulerServiceConfig,
    cancel_token: CancellationToken,
}

impl SchedulerService {
    /// 创建新的调度器服务
    pub fn new(db: DbConnection, config: SchedulerServiceConfig) -> Self {
        let scheduler = Arc::new(AgentScheduler::new(db));
        let executor = Arc::new(AgentExecutor::new());
        let cancel_token = CancellationToken::new();

        Self {
            scheduler,
            executor,
            config,
            cancel_token,
        }
    }

    /// 启动心跳循环
    ///
    /// 在后台 tokio 任务中运行，定期轮询到期任务并执行
    pub fn start(&self, db: DbConnection) {
        if !self.config.enabled {
            tracing::info!("[SchedulerService] 调度器已禁用，跳过启动");
            return;
        }

        let scheduler = self.scheduler.clone();
        let executor = self.executor.clone();
        let config = self.config.clone();
        let cancel_token = self.cancel_token.clone();

        tokio::spawn(async move {
            tracing::info!(
                "[SchedulerService] 启动心跳循环，间隔: {} 秒",
                config.heartbeat_interval_secs
            );

            let mut ticker = interval(Duration::from_secs(config.heartbeat_interval_secs));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = Self::poll_and_execute(
                            &scheduler,
                            &executor,
                            &db,
                            config.max_tasks_per_poll,
                        )
                        .await
                        {
                            tracing::error!("[SchedulerService] 轮询任务失败: {}", e);
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        tracing::info!("[SchedulerService] 收到取消信号，停止心跳循环");
                        break;
                    }
                }
            }

            tracing::info!("[SchedulerService] 心跳循环已停止");
        });
    }

    /// 停止心跳循环
    pub fn stop(&self) {
        tracing::info!("[SchedulerService] 请求停止心跳循环");
        self.cancel_token.cancel();
    }

    /// 轮询并执行到期任务
    async fn poll_and_execute(
        scheduler: &Arc<AgentScheduler>,
        executor: &Arc<AgentExecutor>,
        db: &DbConnection,
        max_tasks: usize,
    ) -> Result<(), String> {
        // 1. 获取到期任务
        let due_tasks = scheduler.get_due_tasks(max_tasks).await?;

        if due_tasks.is_empty() {
            tracing::debug!("[SchedulerService] 没有到期任务");
            return Ok(());
        }

        tracing::info!("[SchedulerService] 发现 {} 个到期任务", due_tasks.len());

        // 2. 执行每个任务
        for task in due_tasks {
            let task_id = task.id.clone();
            let task_name = task.name.clone();

            tracing::info!(
                "[SchedulerService] 开始执行任务: {} ({})",
                task_name,
                task_id
            );

            // 标记为运行中
            if let Err(e) = scheduler.mark_task_running(&task_id).await {
                tracing::error!("[SchedulerService] 标记任务运行失败: {} - {}", task_id, e);
                continue;
            }

            // 执行任务
            match executor.execute(&task, db).await {
                Ok(result) => {
                    // 标记为完成
                    if let Err(e) = scheduler.mark_task_completed(&task_id, Some(result)).await {
                        tracing::error!("[SchedulerService] 标记任务完成失败: {} - {}", task_id, e);
                    }
                }
                Err(e) => {
                    tracing::error!("[SchedulerService] 任务执行失败: {} - {}", task_id, e);

                    // 标记为失败
                    if let Err(mark_err) = scheduler.mark_task_failed(&task_id, e).await {
                        tracing::error!(
                            "[SchedulerService] 标记任务失败失败: {} - {}",
                            task_id,
                            mark_err
                        );
                    }

                    // TODO: 实现重试逻辑
                    // 如果任务可以重试，重新调度任务
                }
            }
        }

        Ok(())
    }

    /// 获取调度器引用
    pub fn scheduler(&self) -> Arc<AgentScheduler> {
        self.scheduler.clone()
    }

    /// 获取执行器引用
    pub fn executor(&self) -> Arc<AgentExecutor> {
        self.executor.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use proxycast_scheduler::{ScheduledTask, SchedulerDao};
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn setup_test_db() -> DbConnection {
        let conn = Connection::open_in_memory().unwrap();
        SchedulerDao::create_tables(&conn).unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_scheduler_service_creation() {
        let db = setup_test_db();
        let config = SchedulerServiceConfig::default();
        let service = SchedulerService::new(db, config);

        assert!(Arc::strong_count(&service.scheduler) >= 1);
        assert!(Arc::strong_count(&service.executor) >= 1);
    }

    #[test]
    fn test_scheduler_service_config_default() {
        let config = SchedulerServiceConfig::default();
        assert_eq!(config.heartbeat_interval_secs, 30);
        assert_eq!(config.max_tasks_per_poll, 10);
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_poll_and_execute_no_tasks() {
        let db = setup_test_db();
        let scheduler = Arc::new(AgentScheduler::new(db.clone()));
        let executor = Arc::new(AgentExecutor::new());

        let result = SchedulerService::poll_and_execute(&scheduler, &executor, &db, 10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poll_and_execute_with_due_task() {
        let db = setup_test_db();
        let scheduler = Arc::new(AgentScheduler::new(db.clone()));
        let executor = Arc::new(AgentExecutor::new());

        // 创建一个到期任务
        let past = Utc::now() - chrono::Duration::hours(1);
        let task = ScheduledTask::new(
            "Test Task".to_string(),
            "agent_chat".to_string(),
            serde_json::json!({"prompt": "test"}),
            "openai".to_string(),
            "gpt-4".to_string(),
            past,
        );

        scheduler.create_task(task).await.unwrap();

        // 轮询并执行（会因为缺少凭证而失败，但不应 panic）
        let result = SchedulerService::poll_and_execute(&scheduler, &executor, &db, 10).await;
        // 即使执行失败，poll_and_execute 也应该返回 Ok
        assert!(result.is_ok());
    }
}
