//! 插件任务执行治理模型
//!
//! 提供统一的任务状态、重试、超时、并发和队列治理能力。

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use crate::event_emit::DynEmitter;

use super::types::PluginError;

/// 插件任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginTaskState {
    Queued,
    Running,
    Retrying,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

impl PluginTaskState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            PluginTaskState::Succeeded
                | PluginTaskState::Failed
                | PluginTaskState::Cancelled
                | PluginTaskState::TimedOut
        )
    }
}

impl FromStr for PluginTaskState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "retrying" => Ok(Self::Retrying),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "timed_out" => Ok(Self::TimedOut),
            _ => Err(format!("未知任务状态: {s}")),
        }
    }
}

/// 任务错误详情
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTaskError {
    pub code: Option<String>,
    pub message: String,
    pub retryable: bool,
}

/// 插件任务执行策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTaskPolicy {
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub max_concurrency_per_plugin: usize,
    pub queue_limit_per_plugin: usize,
}

impl Default for PluginTaskPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            max_retries: 2,
            retry_backoff_ms: 300,
            max_concurrency_per_plugin: 4,
            queue_limit_per_plugin: 100,
        }
    }
}

/// 插件任务记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTaskRecord {
    pub task_id: String,
    pub plugin_id: String,
    pub operation: String,
    pub state: PluginTaskState,
    pub attempt: u32,
    pub max_retries: u32,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub error: Option<PluginTaskError>,
}

impl PluginTaskRecord {
    fn new(task_id: String, plugin_id: String, operation: String, max_retries: u32) -> Self {
        Self {
            task_id,
            plugin_id,
            operation,
            state: PluginTaskState::Queued,
            attempt: 0,
            max_retries,
            started_at: Utc::now(),
            ended_at: None,
            duration_ms: None,
            error: None,
        }
    }

    fn finish_with_success(&mut self, attempt: u32, started: Instant) {
        self.state = PluginTaskState::Succeeded;
        self.attempt = attempt;
        self.ended_at = Some(Utc::now());
        self.duration_ms = Some(started.elapsed().as_millis() as u64);
        self.error = None;
    }

    fn finish_with_failure(
        &mut self,
        state: PluginTaskState,
        attempt: u32,
        started: Instant,
        error: PluginTaskError,
    ) {
        self.state = state;
        self.attempt = attempt;
        self.ended_at = Some(Utc::now());
        self.duration_ms = Some(started.elapsed().as_millis() as u64);
        self.error = Some(error);
    }
}

/// 前端消费的任务事件载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTaskEventPayload {
    pub plugin_id: String,
    pub task_id: String,
    pub operation: String,
    pub state: PluginTaskState,
    pub attempt: u32,
    pub timestamp: String,
    pub error: Option<PluginTaskError>,
}

impl PluginTaskEventPayload {
    fn from_record(record: &PluginTaskRecord) -> Self {
        Self {
            plugin_id: record.plugin_id.clone(),
            task_id: record.task_id.clone(),
            operation: record.operation.clone(),
            state: record.state,
            attempt: record.attempt,
            timestamp: Utc::now().to_rfc3339(),
            error: record.error.clone(),
        }
    }
}

/// 任务失败返回
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTaskFailure {
    pub task_id: String,
    pub state: PluginTaskState,
    pub attempts: u32,
    pub message: String,
    pub retryable: bool,
}

impl PluginTaskFailure {
    fn new(
        task_id: String,
        state: PluginTaskState,
        attempts: u32,
        message: String,
        retryable: bool,
    ) -> Self {
        Self {
            task_id,
            state,
            attempts,
            message,
            retryable,
        }
    }
}

/// 插件队列统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginQueueStats {
    pub plugin_id: String,
    pub running: usize,
    pub waiting: usize,
    pub rejected: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub timed_out: u64,
}

#[derive(Default)]
struct QueueMetrics {
    running: AtomicUsize,
    waiting: AtomicUsize,
    rejected: AtomicU64,
    completed: AtomicU64,
    failed: AtomicU64,
    cancelled: AtomicU64,
    timed_out: AtomicU64,
}

impl QueueMetrics {
    fn snapshot(&self, plugin_id: String) -> PluginQueueStats {
        PluginQueueStats {
            plugin_id,
            running: self.running.load(Ordering::SeqCst),
            waiting: self.waiting.load(Ordering::SeqCst),
            rejected: self.rejected.load(Ordering::SeqCst),
            completed: self.completed.load(Ordering::SeqCst),
            failed: self.failed.load(Ordering::SeqCst),
            cancelled: self.cancelled.load(Ordering::SeqCst),
            timed_out: self.timed_out.load(Ordering::SeqCst),
        }
    }
}

struct RunningGuard {
    metrics: Arc<QueueMetrics>,
}

impl RunningGuard {
    fn new(metrics: Arc<QueueMetrics>) -> Self {
        Self { metrics }
    }
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.metrics.running.fetch_sub(1, Ordering::SeqCst);
    }
}

/// 插件任务跟踪器
pub struct PluginTaskTracker {
    tasks: DashMap<String, PluginTaskRecord>,
    semaphores: DashMap<String, Arc<Semaphore>>,
    queue_metrics: DashMap<String, Arc<QueueMetrics>>,
    cancel_flags: DashMap<String, Arc<AtomicBool>>,
    retention_limit: usize,
    emitter: Arc<RwLock<Option<DynEmitter>>>,
}

impl Default for PluginTaskTracker {
    fn default() -> Self {
        Self::new(2_000)
    }
}

impl PluginTaskTracker {
    pub fn new(retention_limit: usize) -> Self {
        Self {
            tasks: DashMap::new(),
            semaphores: DashMap::new(),
            queue_metrics: DashMap::new(),
            cancel_flags: DashMap::new(),
            retention_limit: retention_limit.max(100),
            emitter: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_emitter(&self, emitter: DynEmitter) {
        let mut guard = self.emitter.write().await;
        *guard = Some(emitter);
    }

    pub async fn clear_emitter(&self) {
        let mut guard = self.emitter.write().await;
        *guard = None;
    }

    pub fn get_task(&self, task_id: &str) -> Option<PluginTaskRecord> {
        self.tasks.get(task_id).map(|entry| entry.value().clone())
    }

    pub fn list_tasks(
        &self,
        plugin_id: Option<&str>,
        state: Option<PluginTaskState>,
        limit: usize,
    ) -> Vec<PluginTaskRecord> {
        let mut records: Vec<PluginTaskRecord> = self
            .tasks
            .iter()
            .filter_map(|entry| {
                let record = entry.value();
                if let Some(plugin_id_filter) = plugin_id {
                    if record.plugin_id != plugin_id_filter {
                        return None;
                    }
                }
                if let Some(state_filter) = state {
                    if record.state != state_filter {
                        return None;
                    }
                }
                Some(record.clone())
            })
            .collect();

        records.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        records.truncate(limit.max(1));
        records
    }

    pub fn cancel_task(&self, task_id: &str) -> bool {
        let Some(flag) = self.cancel_flags.get(task_id) else {
            return false;
        };
        flag.store(true, Ordering::SeqCst);
        true
    }

    pub fn queue_stats(&self, plugin_id: Option<&str>) -> Vec<PluginQueueStats> {
        let mut items = Vec::new();
        for entry in &self.queue_metrics {
            if let Some(plugin_filter) = plugin_id {
                if entry.key() != plugin_filter {
                    continue;
                }
            }
            items.push(entry.value().snapshot(entry.key().clone()));
        }
        items.sort_by(|a, b| a.plugin_id.cmp(&b.plugin_id));
        items
    }

    pub async fn execute<T, F, Fut>(
        &self,
        plugin_id: &str,
        operation: &str,
        mut policy: PluginTaskPolicy,
        mut operation_fn: F,
    ) -> Result<T, PluginTaskFailure>
    where
        T: Send + 'static,
        F: FnMut(u32) -> Fut + Send,
        Fut: Future<Output = Result<T, PluginError>> + Send,
    {
        if policy.max_concurrency_per_plugin == 0 {
            policy.max_concurrency_per_plugin = 1;
        }
        if policy.queue_limit_per_plugin == 0 {
            policy.queue_limit_per_plugin = 1;
        }
        if policy.timeout_ms == 0 {
            policy.timeout_ms = 1;
        }

        let task_id = Uuid::new_v4().to_string();
        let total_started = Instant::now();
        let mut record = PluginTaskRecord::new(
            task_id.clone(),
            plugin_id.to_string(),
            operation.to_string(),
            policy.max_retries,
        );
        self.upsert_task(record.clone());
        self.emit_task_event(&record).await;

        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.cancel_flags
            .insert(task_id.clone(), Arc::clone(&cancel_flag));

        let semaphore = self
            .semaphores
            .entry(plugin_id.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(policy.max_concurrency_per_plugin)))
            .clone();
        let metrics = self
            .queue_metrics
            .entry(plugin_id.to_string())
            .or_insert_with(|| Arc::new(QueueMetrics::default()))
            .clone();

        let waiting_now = metrics.waiting.fetch_add(1, Ordering::SeqCst) + 1;
        if waiting_now > policy.queue_limit_per_plugin {
            metrics.waiting.fetch_sub(1, Ordering::SeqCst);
            metrics.rejected.fetch_add(1, Ordering::SeqCst);

            let error = PluginTaskError {
                code: Some("QUEUE_LIMIT_EXCEEDED".to_string()),
                message: format!(
                    "插件 {plugin_id} 队列已满 (limit={})",
                    policy.queue_limit_per_plugin
                ),
                retryable: false,
            };
            record.finish_with_failure(PluginTaskState::Failed, 0, total_started, error.clone());
            self.upsert_task(record.clone());
            self.cancel_flags.remove(&task_id);
            self.emit_task_event(&record).await;
            return Err(PluginTaskFailure::new(
                task_id,
                PluginTaskState::Failed,
                0,
                error.message,
                false,
            ));
        }

        let permit = match semaphore.acquire_owned().await {
            Ok(permit) => permit,
            Err(err) => {
                metrics.waiting.fetch_sub(1, Ordering::SeqCst);
                let error = PluginTaskError {
                    code: Some("SEMAPHORE_CLOSED".to_string()),
                    message: format!("无法获取插件执行许可: {err}"),
                    retryable: true,
                };
                record.finish_with_failure(
                    PluginTaskState::Failed,
                    0,
                    total_started,
                    error.clone(),
                );
                self.upsert_task(record.clone());
                self.cancel_flags.remove(&task_id);
                self.emit_task_event(&record).await;
                return Err(PluginTaskFailure::new(
                    task_id,
                    PluginTaskState::Failed,
                    0,
                    error.message,
                    true,
                ));
            }
        };
        metrics.waiting.fetch_sub(1, Ordering::SeqCst);
        metrics.running.fetch_add(1, Ordering::SeqCst);
        let running_guard = RunningGuard::new(Arc::clone(&metrics));

        if cancel_flag.load(Ordering::SeqCst) {
            let error = PluginTaskError {
                code: Some("TASK_CANCELLED".to_string()),
                message: "任务已取消".to_string(),
                retryable: false,
            };
            record.finish_with_failure(PluginTaskState::Cancelled, 0, total_started, error.clone());
            metrics.cancelled.fetch_add(1, Ordering::SeqCst);
            self.upsert_task(record.clone());
            self.cancel_flags.remove(&task_id);
            self.emit_task_event(&record).await;
            drop(permit);
            drop(running_guard);
            return Err(PluginTaskFailure::new(
                task_id,
                PluginTaskState::Cancelled,
                0,
                error.message,
                false,
            ));
        }

        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            record.state = if attempt == 1 {
                PluginTaskState::Running
            } else {
                PluginTaskState::Retrying
            };
            record.attempt = attempt;
            record.error = None;
            self.upsert_task(record.clone());
            self.emit_task_event(&record).await;

            let timed_result = timeout(
                Duration::from_millis(policy.timeout_ms),
                operation_fn(attempt),
            )
            .await;

            match timed_result {
                Ok(Ok(value)) => {
                    record.finish_with_success(attempt, total_started);
                    metrics.completed.fetch_add(1, Ordering::SeqCst);
                    self.upsert_task(record.clone());
                    self.cancel_flags.remove(&task_id);
                    self.emit_task_event(&record).await;
                    drop(permit);
                    drop(running_guard);
                    return Ok(value);
                }
                Ok(Err(err)) => {
                    let retryable = is_retryable_error(&err);
                    let can_retry = retryable
                        && attempt <= policy.max_retries
                        && !cancel_flag.load(Ordering::SeqCst);

                    if can_retry {
                        let backoff = backoff_duration(policy.retry_backoff_ms, attempt);
                        sleep(backoff).await;
                        continue;
                    }

                    let state = if cancel_flag.load(Ordering::SeqCst) {
                        PluginTaskState::Cancelled
                    } else {
                        PluginTaskState::Failed
                    };
                    let error = PluginTaskError {
                        code: classify_error_code(&err),
                        message: err.to_string(),
                        retryable,
                    };
                    record.finish_with_failure(state, attempt, total_started, error.clone());
                    match state {
                        PluginTaskState::Cancelled => {
                            metrics.cancelled.fetch_add(1, Ordering::SeqCst);
                        }
                        PluginTaskState::Failed => {
                            metrics.failed.fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {}
                    }
                    self.upsert_task(record.clone());
                    self.cancel_flags.remove(&task_id);
                    self.emit_task_event(&record).await;
                    drop(permit);
                    drop(running_guard);
                    return Err(PluginTaskFailure::new(
                        task_id,
                        state,
                        attempt,
                        error.message,
                        retryable,
                    ));
                }
                Err(_) => {
                    let can_retry =
                        attempt <= policy.max_retries && !cancel_flag.load(Ordering::SeqCst);
                    if can_retry {
                        let backoff = backoff_duration(policy.retry_backoff_ms, attempt);
                        sleep(backoff).await;
                        continue;
                    }
                    let state = if cancel_flag.load(Ordering::SeqCst) {
                        PluginTaskState::Cancelled
                    } else {
                        PluginTaskState::TimedOut
                    };
                    let error = PluginTaskError {
                        code: Some(if state == PluginTaskState::TimedOut {
                            "TIMEOUT".to_string()
                        } else {
                            "TASK_CANCELLED".to_string()
                        }),
                        message: if state == PluginTaskState::TimedOut {
                            format!("执行超时: {}ms", policy.timeout_ms)
                        } else {
                            "任务已取消".to_string()
                        },
                        retryable: state == PluginTaskState::TimedOut,
                    };
                    record.finish_with_failure(state, attempt, total_started, error.clone());
                    match state {
                        PluginTaskState::TimedOut => {
                            metrics.timed_out.fetch_add(1, Ordering::SeqCst);
                        }
                        PluginTaskState::Cancelled => {
                            metrics.cancelled.fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {}
                    }
                    self.upsert_task(record.clone());
                    self.cancel_flags.remove(&task_id);
                    self.emit_task_event(&record).await;
                    drop(permit);
                    drop(running_guard);
                    return Err(PluginTaskFailure::new(
                        task_id,
                        state,
                        attempt,
                        error.message,
                        state == PluginTaskState::TimedOut,
                    ));
                }
            }
        }
    }

    fn upsert_task(&self, record: PluginTaskRecord) {
        self.tasks.insert(record.task_id.clone(), record);
        self.trim_retention();
    }

    fn trim_retention(&self) {
        if self.tasks.len() <= self.retention_limit {
            return;
        }

        while self.tasks.len() > self.retention_limit {
            let oldest_id = self
                .tasks
                .iter()
                .min_by_key(|entry| entry.value().started_at)
                .map(|entry| entry.key().clone());
            let Some(oldest_id) = oldest_id else {
                break;
            };
            self.tasks.remove(&oldest_id);
            self.cancel_flags.remove(&oldest_id);
        }
    }

    async fn emit_task_event(&self, record: &PluginTaskRecord) {
        let payload = PluginTaskEventPayload::from_record(record);
        let Ok(value) = serde_json::to_value(payload) else {
            return;
        };

        let emitter = self.emitter.read().await.clone();
        if let Some(emitter) = emitter {
            let _ = emitter.emit_event("plugin-task-event", &value);
        }
    }
}

fn backoff_duration(base_ms: u64, attempt: u32) -> Duration {
    let factor = 2_u64.saturating_pow(attempt.saturating_sub(1));
    Duration::from_millis(base_ms.max(1).saturating_mul(factor))
}

fn classify_error_code(err: &PluginError) -> Option<String> {
    match err {
        PluginError::Timeout { .. } => Some("TIMEOUT".to_string()),
        PluginError::Disabled(_) => Some("PLUGIN_DISABLED".to_string()),
        PluginError::NotFound(_) => Some("PLUGIN_NOT_FOUND".to_string()),
        PluginError::ConfigError(_) => Some("CONFIG_ERROR".to_string()),
        PluginError::LoadError(_) => Some("LOAD_ERROR".to_string()),
        PluginError::InitError(_) => Some("INIT_ERROR".to_string()),
        PluginError::ExecutionError { message, .. } => {
            if message.contains("401") || message.contains("403") {
                Some("AUTH_ERROR".to_string())
            } else if message.contains("429") {
                Some("RATE_LIMIT".to_string())
            } else if message.contains("500")
                || message.contains("502")
                || message.contains("503")
                || message.contains("504")
            {
                Some("UPSTREAM_5XX".to_string())
            } else {
                Some("EXECUTION_ERROR".to_string())
            }
        }
        _ => Some("UNKNOWN".to_string()),
    }
}

fn is_retryable_error(err: &PluginError) -> bool {
    match err {
        PluginError::Timeout { .. } => true,
        PluginError::ExecutionError { message, .. } => {
            let lower = message.to_lowercase();
            message.contains("429")
                || message.contains("500")
                || message.contains("502")
                || message.contains("503")
                || message.contains("504")
                || lower.contains("timeout")
                || lower.contains("temporar")
                || lower.contains("connection")
                || lower.contains("network")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_execute_success_and_record_terminal_state() {
        let tracker = PluginTaskTracker::new(100);
        let policy = PluginTaskPolicy::default();

        let result = tracker
            .execute("demo-plugin", "on_request", policy, |_attempt| async move {
                Ok::<_, PluginError>("ok".to_string())
            })
            .await
            .expect("执行应成功");

        assert_eq!(result, "ok");
        let tasks = tracker.list_tasks(Some("demo-plugin"), None, 10);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].state, PluginTaskState::Succeeded);
        assert_eq!(tasks[0].attempt, 1);
    }

    #[tokio::test]
    async fn test_retry_then_success() {
        let tracker = PluginTaskTracker::new(100);
        let policy = PluginTaskPolicy {
            max_retries: 2,
            retry_backoff_ms: 1,
            ..PluginTaskPolicy::default()
        };
        let counter = Arc::new(Mutex::new(0_u32));

        let result = tracker
            .execute("retry-plugin", "on_response", policy, {
                let counter = Arc::clone(&counter);
                move |_attempt| {
                    let counter = Arc::clone(&counter);
                    async move {
                        let mut lock = counter.lock().await;
                        *lock += 1;
                        if *lock < 2 {
                            Err(PluginError::ExecutionError {
                                plugin_name: "retry-plugin".to_string(),
                                message: "503 upstream unavailable".to_string(),
                            })
                        } else {
                            Ok::<_, PluginError>("recovered".to_string())
                        }
                    }
                }
            })
            .await
            .expect("应在重试后成功");

        assert_eq!(result, "recovered");
        let tasks = tracker.list_tasks(Some("retry-plugin"), None, 10);
        assert_eq!(tasks[0].state, PluginTaskState::Succeeded);
        assert_eq!(tasks[0].attempt, 2);
    }

    #[tokio::test]
    async fn test_timeout_to_terminal_state() {
        let tracker = PluginTaskTracker::new(100);
        let policy = PluginTaskPolicy {
            timeout_ms: 30,
            max_retries: 0,
            ..PluginTaskPolicy::default()
        };

        let result = tracker
            .execute(
                "timeout-plugin",
                "on_error",
                policy,
                |_attempt| async move {
                    sleep(Duration::from_millis(80)).await;
                    Ok::<_, PluginError>("late".to_string())
                },
            )
            .await;

        assert!(result.is_err());
        let err = result.expect_err("应超时失败");
        assert_eq!(err.state, PluginTaskState::TimedOut);
        let tasks = tracker.list_tasks(Some("timeout-plugin"), None, 10);
        assert_eq!(tasks[0].state, PluginTaskState::TimedOut);
    }

    #[tokio::test]
    async fn test_queue_limit_rejection() {
        let tracker = Arc::new(PluginTaskTracker::new(100));
        let policy = PluginTaskPolicy {
            max_concurrency_per_plugin: 1,
            queue_limit_per_plugin: 1,
            timeout_ms: 500,
            max_retries: 0,
            ..PluginTaskPolicy::default()
        };

        let tracker_a = Arc::clone(&tracker);
        let policy_a = policy.clone();
        let t1 = tokio::spawn(async move {
            tracker_a
                .execute(
                    "queue-plugin",
                    "on_request",
                    policy_a,
                    |_attempt| async move {
                        sleep(Duration::from_millis(150)).await;
                        Ok::<_, PluginError>("t1".to_string())
                    },
                )
                .await
        });

        sleep(Duration::from_millis(20)).await;

        let tracker_b = Arc::clone(&tracker);
        let policy_b = policy.clone();
        let t2 = tokio::spawn(async move {
            tracker_b
                .execute(
                    "queue-plugin",
                    "on_request",
                    policy_b,
                    |_attempt| async move {
                        sleep(Duration::from_millis(80)).await;
                        Ok::<_, PluginError>("t2".to_string())
                    },
                )
                .await
        });

        sleep(Duration::from_millis(20)).await;

        let t3 = tracker
            .execute(
                "queue-plugin",
                "on_request",
                policy,
                |_attempt| async move { Ok::<_, PluginError>("t3".to_string()) },
            )
            .await;

        let r1 = t1.await.expect("join t1");
        let r2 = t2.await.expect("join t2");
        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(t3.is_err());

        let stats = tracker.queue_stats(Some("queue-plugin"));
        assert_eq!(stats.len(), 1);
        assert!(stats[0].rejected >= 1);
    }
}
