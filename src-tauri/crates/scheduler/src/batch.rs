//! 批量任务定义
//!
//! 定义批量任务相关的数据结构和状态

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 批量任务选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptions {
    /// 并发数量 (默认为 3)
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// 失败后是否继续 (默认为 true)
    #[serde(default = "default_continue_on_error")]
    pub continue_on_error: bool,

    /// 重试次数 (默认为 0)
    #[serde(default)]
    pub retry_count: usize,

    /// 任务超时时间(秒) (默认为 120)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_concurrency() -> usize {
    3
}

fn default_continue_on_error() -> bool {
    true
}

fn default_timeout() -> u64 {
    120
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            continue_on_error: default_continue_on_error(),
            retry_count: 0,
            timeout_seconds: default_timeout(),
        }
    }
}

/// 单个任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    /// 任务 ID (可选,如果不提供则自动生成)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,

    /// 模板变量
    pub variables: HashMap<String, String>,

    /// 任务元数据 (用于追踪和识别)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// 单个任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// 任务 ID
    pub task_id: Uuid,

    /// 任务状态
    pub status: TaskStatus,

    /// 响应内容 (如果成功)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// 错误信息 (如果失败)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// 使用 token 数
    #[serde(default)]
    pub usage: TokenUsage,

    /// 开始时间
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// 完成时间
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// 等待中
    Pending,

    /// 运行中
    Running,

    /// 已完成
    Completed,

    /// 失败
    Failed,

    /// 已取消
    Cancelled,
}

/// Token 使用统计
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 输入 token 数
    #[serde(default)]
    pub prompt_tokens: u32,

    /// 输出 token 数
    #[serde(default)]
    pub completion_tokens: u32,

    /// 总 token 数
    #[serde(default)]
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// 批量任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTask {
    /// 批量任务 ID
    pub id: Uuid,

    /// 批量任务名称
    pub name: String,

    /// 任务模板 ID
    pub template_id: Uuid,

    /// 任务列表
    pub tasks: Vec<TaskDefinition>,

    /// 批量任务选项
    #[serde(default)]
    pub options: BatchOptions,

    /// 批量任务状态
    pub status: BatchTaskStatus,

    /// 任务结果
    #[serde(default)]
    pub results: Vec<TaskResult>,

    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// 开始时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,

    /// 完成时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 批量任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchTaskStatus {
    /// 等待中
    Pending,

    /// 运行中
    Running,

    /// 已完成
    Completed,

    /// 部分完成 (部分任务失败)
    PartiallyCompleted,

    /// 失败 (所有任务失败)
    Failed,

    /// 已取消
    Cancelled,
}

impl BatchTask {
    /// 创建新的批量任务
    pub fn new(
        name: String,
        template_id: Uuid,
        tasks: Vec<TaskDefinition>,
        options: BatchOptions,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            template_id,
            tasks,
            options,
            status: BatchTaskStatus::Pending,
            results: Vec::new(),
            created_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    /// 获取进度信息
    pub fn get_progress(&self) -> (usize, usize, usize) {
        // (总数, 成功数, 失败数)
        let total = self.tasks.len();
        let completed = self
            .results
            .iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .count();
        let failed = self
            .results
            .iter()
            .filter(|r| r.status == TaskStatus::Failed)
            .count();
        (total, completed, failed)
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> BatchTaskStatistics {
        let (total, completed, failed) = self.get_progress();
        let running = self
            .results
            .iter()
            .filter(|r| r.status == TaskStatus::Running)
            .count();
        let total_tokens: TokenUsage =
            self.results
                .iter()
                .fold(TokenUsage::default(), |mut acc, r| {
                    acc.prompt_tokens += r.usage.prompt_tokens;
                    acc.completion_tokens += r.usage.completion_tokens;
                    acc.total_tokens += r.usage.total_tokens;
                    acc
                });

        BatchTaskStatistics {
            total_tasks: total,
            completed_tasks: completed,
            failed_tasks: failed,
            running_tasks: running,
            pending_tasks: total - completed - failed - running,
            total_tokens,
        }
    }
}

/// 批量任务统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskStatistics {
    /// 总任务数
    pub total_tasks: usize,

    /// 已完成任务数
    pub completed_tasks: usize,

    /// 失败任务数
    pub failed_tasks: usize,

    /// 运行中任务数
    pub running_tasks: usize,

    /// 等待中任务数
    pub pending_tasks: usize,

    /// 总 token 使用量
    pub total_tokens: TokenUsage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_options_default() {
        let options = BatchOptions::default();
        assert_eq!(options.concurrency, 3);
        assert_eq!(options.continue_on_error, true);
        assert_eq!(options.retry_count, 0);
        assert_eq!(options.timeout_seconds, 120);
    }

    #[test]
    fn test_batch_task_creation() {
        let tasks = vec![
            TaskDefinition {
                id: None,
                variables: {
                    let mut map = HashMap::new();
                    map.insert("content".to_string(), "测试1".to_string());
                    map
                },
                metadata: HashMap::new(),
            },
            TaskDefinition {
                id: None,
                variables: {
                    let mut map = HashMap::new();
                    map.insert("content".to_string(), "测试2".to_string());
                    map
                },
                metadata: HashMap::new(),
            },
        ];

        let batch_task = BatchTask::new(
            "测试批量任务".to_string(),
            Uuid::new_v4(),
            tasks,
            BatchOptions::default(),
        );

        assert_eq!(batch_task.name, "测试批量任务");
        assert_eq!(batch_task.tasks.len(), 2);
        assert_eq!(batch_task.status, BatchTaskStatus::Pending);
    }

    #[test]
    fn test_get_progress() {
        let mut batch_task = BatchTask::new(
            "测试".to_string(),
            Uuid::new_v4(),
            vec![
                TaskDefinition {
                    id: Some(Uuid::new_v4()),
                    variables: HashMap::new(),
                    metadata: HashMap::new(),
                },
                TaskDefinition {
                    id: Some(Uuid::new_v4()),
                    variables: HashMap::new(),
                    metadata: HashMap::new(),
                },
                TaskDefinition {
                    id: Some(Uuid::new_v4()),
                    variables: HashMap::new(),
                    metadata: HashMap::new(),
                },
            ],
            BatchOptions::default(),
        );

        // 添加一些结果
        batch_task.results.push(TaskResult {
            task_id: batch_task.tasks[0].id.unwrap(),
            status: TaskStatus::Completed,
            content: Some("完成".to_string()),
            error: None,
            usage: TokenUsage::default(),
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        });

        batch_task.results.push(TaskResult {
            task_id: batch_task.tasks[1].id.unwrap(),
            status: TaskStatus::Failed,
            content: None,
            error: Some("失败".to_string()),
            usage: TokenUsage::default(),
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        });

        let (total, completed, failed) = batch_task.get_progress();
        assert_eq!(total, 3);
        assert_eq!(completed, 1);
        assert_eq!(failed, 1);
    }
}
