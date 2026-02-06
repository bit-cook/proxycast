//! Tauri 执行回调实现
//!
//! 实现 aster-rust 的 ExecutionCallback trait，通过 Tauri 事件系统向前端发送进度更新。
//!
//! ## 事件类型
//! - `skill:step_start`: 步骤开始
//! - `skill:step_complete`: 步骤完成
//! - `skill:step_error`: 步骤错误
//! - `skill:complete`: 执行完成
//!
//! ## 使用示例
//! ```ignore
//! let callback = TauriExecutionCallback::new(app_handle, "exec-123".to_string());
//! callback.on_step_start("step-1", "数据处理", 1, 3);
//! ```

use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{AppHandle, Emitter};

/// 步骤开始事件 Payload
#[derive(Debug, Clone, Serialize)]
pub struct StepStartPayload {
    /// 执行 ID
    pub execution_id: String,
    /// 步骤 ID
    pub step_id: String,
    /// 步骤名称
    pub step_name: String,
    /// 当前步骤序号（从 1 开始）
    pub current_step: usize,
    /// 总步骤数
    pub total_steps: usize,
}

/// 步骤完成事件 Payload
#[derive(Debug, Clone, Serialize)]
pub struct StepCompletePayload {
    /// 执行 ID
    pub execution_id: String,
    /// 步骤 ID
    pub step_id: String,
    /// 步骤输出
    pub output: String,
}

/// 步骤错误事件 Payload
#[derive(Debug, Clone, Serialize)]
pub struct StepErrorPayload {
    /// 执行 ID
    pub execution_id: String,
    /// 步骤 ID
    pub step_id: String,
    /// 错误信息
    pub error: String,
    /// 是否会重试
    pub will_retry: bool,
}

/// 执行完成事件 Payload
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionCompletePayload {
    /// 执行 ID
    pub execution_id: String,
    /// 是否成功
    pub success: bool,
    /// 最终输出（成功时）
    pub output: Option<String>,
    /// 错误信息（失败时）
    pub error: Option<String>,
}

/// Tauri 事件名称常量
pub mod events {
    /// 步骤开始事件
    pub const STEP_START: &str = "skill:step_start";
    /// 步骤完成事件
    pub const STEP_COMPLETE: &str = "skill:step_complete";
    /// 步骤错误事件
    pub const STEP_ERROR: &str = "skill:step_error";
    /// 执行完成事件
    pub const COMPLETE: &str = "skill:complete";
}

/// ExecutionCallback Trait
///
/// 定义 Skill 执行过程中的回调接口。
/// 应用层需要实现此 trait 以接收执行进度更新。
pub trait ExecutionCallback: Send + Sync {
    /// 步骤开始回调
    ///
    /// # 参数
    /// - `step_id`: 步骤 ID
    /// - `step_name`: 步骤名称
    /// - `current_step`: 当前步骤序号（从 1 开始）
    /// - `total_steps`: 总步骤数
    fn on_step_start(
        &self,
        step_id: &str,
        step_name: &str,
        current_step: usize,
        total_steps: usize,
    );

    /// 步骤完成回调
    ///
    /// # 参数
    /// - `step_id`: 步骤 ID
    /// - `output`: 步骤输出
    fn on_step_complete(&self, step_id: &str, output: &str);

    /// 步骤错误回调
    ///
    /// # 参数
    /// - `step_id`: 步骤 ID
    /// - `error`: 错误信息
    /// - `will_retry`: 是否会重试
    fn on_step_error(&self, step_id: &str, error: &str, will_retry: bool);

    /// 执行完成回调
    ///
    /// # 参数
    /// - `success`: 是否成功
    /// - `final_output`: 最终输出（成功时）
    /// - `error`: 错误信息（失败时）
    fn on_complete(&self, success: bool, final_output: Option<&str>, error: Option<&str>);
}

/// Tauri 执行回调
///
/// 通过 Tauri 事件系统向前端发送 Skill 执行进度更新。
/// 实现 aster-rust 定义的 ExecutionCallback trait。
pub struct TauriExecutionCallback {
    /// Tauri AppHandle
    app_handle: AppHandle,
    /// 执行 ID（用于区分多个并发执行）
    execution_id: String,
    /// 当前步骤计数器（用于跟踪步骤序号）
    current_step: AtomicUsize,
}

impl TauriExecutionCallback {
    /// 创建新的 TauriExecutionCallback 实例
    ///
    /// # Arguments
    /// * `app_handle` - Tauri AppHandle
    /// * `execution_id` - 执行 ID，用于区分多个并发执行
    pub fn new(app_handle: AppHandle, execution_id: String) -> Self {
        Self {
            app_handle,
            execution_id,
            current_step: AtomicUsize::new(0),
        }
    }

    /// 获取执行 ID
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// 获取当前步骤序号
    pub fn current_step(&self) -> usize {
        self.current_step.load(Ordering::SeqCst)
    }
}

/// ExecutionCallback trait 实现
///
/// 通过 Tauri 事件系统向前端发送进度更新。
///
/// # Requirements
/// - 2.2: on_step_start 发送 "skill:step_start" 事件
/// - 2.3: on_step_complete 发送 "skill:step_complete" 事件
/// - 2.4: on_step_error 发送 "skill:step_error" 事件
/// - 2.5: on_complete 发送 "skill:complete" 事件
impl ExecutionCallback for TauriExecutionCallback {
    /// 步骤开始回调
    ///
    /// 发送 "skill:step_start" Tauri 事件到前端。
    ///
    /// # Requirements
    /// - 2.2: WHEN on_step_start is called, emit a "skill:step_start" Tauri event
    fn on_step_start(
        &self,
        step_id: &str,
        step_name: &str,
        current_step: usize,
        total_steps: usize,
    ) {
        // 更新当前步骤计数器
        self.current_step.store(current_step, Ordering::SeqCst);

        let payload = StepStartPayload {
            execution_id: self.execution_id.clone(),
            step_id: step_id.to_string(),
            step_name: step_name.to_string(),
            current_step,
            total_steps,
        };

        tracing::info!(
            "[TauriExecutionCallback] 步骤开始: execution_id={}, step_id={}, step_name={}, {}/{}",
            self.execution_id,
            step_id,
            step_name,
            current_step,
            total_steps
        );

        if let Err(e) = self.app_handle.emit(events::STEP_START, &payload) {
            tracing::error!(
                "[TauriExecutionCallback] 发送 {} 事件失败: {}",
                events::STEP_START,
                e
            );
        }
    }

    /// 步骤完成回调
    ///
    /// 发送 "skill:step_complete" Tauri 事件到前端。
    ///
    /// # Requirements
    /// - 2.3: WHEN on_step_complete is called, emit a "skill:step_complete" Tauri event
    fn on_step_complete(&self, step_id: &str, output: &str) {
        let payload = StepCompletePayload {
            execution_id: self.execution_id.clone(),
            step_id: step_id.to_string(),
            output: output.to_string(),
        };

        tracing::info!(
            "[TauriExecutionCallback] 步骤完成: execution_id={}, step_id={}, output_len={}",
            self.execution_id,
            step_id,
            output.len()
        );

        if let Err(e) = self.app_handle.emit(events::STEP_COMPLETE, &payload) {
            tracing::error!(
                "[TauriExecutionCallback] 发送 {} 事件失败: {}",
                events::STEP_COMPLETE,
                e
            );
        }
    }

    /// 步骤错误回调
    ///
    /// 发送 "skill:step_error" Tauri 事件到前端。
    ///
    /// # Requirements
    /// - 2.4: WHEN on_step_error is called, emit a "skill:step_error" Tauri event
    fn on_step_error(&self, step_id: &str, error: &str, will_retry: bool) {
        let payload = StepErrorPayload {
            execution_id: self.execution_id.clone(),
            step_id: step_id.to_string(),
            error: error.to_string(),
            will_retry,
        };

        tracing::warn!(
            "[TauriExecutionCallback] 步骤错误: execution_id={}, step_id={}, error={}, will_retry={}",
            self.execution_id,
            step_id,
            error,
            will_retry
        );

        if let Err(e) = self.app_handle.emit(events::STEP_ERROR, &payload) {
            tracing::error!(
                "[TauriExecutionCallback] 发送 {} 事件失败: {}",
                events::STEP_ERROR,
                e
            );
        }
    }

    /// 执行完成回调
    ///
    /// 发送 "skill:complete" Tauri 事件到前端。
    ///
    /// # Requirements
    /// - 2.5: WHEN on_complete is called, emit a "skill:complete" Tauri event
    fn on_complete(&self, success: bool, final_output: Option<&str>, error: Option<&str>) {
        let payload = ExecutionCompletePayload {
            execution_id: self.execution_id.clone(),
            success,
            output: final_output.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
        };

        if success {
            tracing::info!(
                "[TauriExecutionCallback] 执行完成: execution_id={}, success=true, output_len={}",
                self.execution_id,
                final_output.map(|s| s.len()).unwrap_or(0)
            );
        } else {
            tracing::warn!(
                "[TauriExecutionCallback] 执行失败: execution_id={}, error={:?}",
                self.execution_id,
                error
            );
        }

        if let Err(e) = self.app_handle.emit(events::COMPLETE, &payload) {
            tracing::error!(
                "[TauriExecutionCallback] 发送 {} 事件失败: {}",
                events::COMPLETE,
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: 在 Task 1.5 中添加属性测试
}
