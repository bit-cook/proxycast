//! 重试机制实现
//!
//! 提供带指数退避和抖动的重试逻辑

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::Duration;

/// 可重试的 HTTP 状态码
pub const RETRYABLE_STATUS_CODES: &[u16] = &[408, 429, 500, 502, 503, 504];

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 基础延迟（毫秒）
    pub base_delay_ms: u64,
    /// 最大延迟（毫秒）
    pub max_delay_ms: u64,
    /// 可重试的状态码
    #[serde(default = "default_retryable_codes")]
    pub retryable_codes: Vec<u16>,
}

fn default_retryable_codes() -> Vec<u16> {
    RETRYABLE_STATUS_CODES.to_vec()
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            retryable_codes: default_retryable_codes(),
        }
    }
}

impl RetryConfig {
    /// 创建新的重试配置
    pub fn new(max_retries: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
            retryable_codes: default_retryable_codes(),
        }
    }

    /// 检查状态码是否可重试
    pub fn is_retryable(&self, status_code: u16) -> bool {
        self.retryable_codes.contains(&status_code)
    }
}

/// 重试错误
#[derive(Debug, Clone)]
pub struct RetryError {
    /// 尝试次数（包括初始请求）
    pub attempts: u32,
    /// 最后一次错误信息
    pub last_error: String,
    /// 最后一次状态码（如果有）
    pub last_status_code: Option<u16>,
}

impl std::fmt::Display for RetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "重试耗尽: 尝试 {} 次后失败 - {}",
            self.attempts, self.last_error
        )
    }
}

impl std::error::Error for RetryError {}

/// 重试器
#[derive(Debug, Clone)]
pub struct Retrier {
    config: RetryConfig,
}

impl Retrier {
    /// 创建新的重试器
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建重试器
    pub fn with_defaults() -> Self {
        Self::new(RetryConfig::default())
    }

    /// 获取配置
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// 计算第 N 次重试的退避时间（指数退避 + 抖动）
    ///
    /// 公式: min(base_delay * 2^attempt + jitter, max_delay)
    /// 其中 jitter 是 [0, base_delay) 范围内的随机值
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        self.backoff_delay_with_jitter(attempt, rand_jitter_factor())
    }

    /// 计算退避时间（可指定抖动因子，用于测试）
    ///
    /// jitter_factor 应在 [0.0, 1.0) 范围内
    pub fn backoff_delay_with_jitter(&self, attempt: u32, jitter_factor: f64) -> Duration {
        let base = self.config.base_delay_ms as f64;
        let max = self.config.max_delay_ms as f64;

        // 指数退避: base * 2^attempt
        let exponential = base * 2_f64.powi(attempt as i32);

        // 抖动: [0, base) 范围内的随机值
        let jitter = base * jitter_factor.clamp(0.0, 1.0);

        // 总延迟，不超过最大值
        let delay = (exponential + jitter).min(max);

        Duration::from_millis(delay as u64)
    }

    /// 带重试执行异步操作
    ///
    /// 操作函数返回 `Result<T, (String, Option<u16>)>`，
    /// 其中错误元组包含错误信息和可选的状态码
    pub async fn execute<F, Fut, T>(&self, mut operation: F) -> Result<T, RetryError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, (String, Option<u16>)>>,
    {
        let mut attempts = 0u32;
        let mut last_error;
        let mut last_status_code;

        loop {
            attempts += 1;

            match operation().await {
                Ok(result) => return Ok(result),
                Err((error, status_code)) => {
                    last_error = error;
                    last_status_code = status_code;

                    // 检查是否应该重试
                    let should_retry = if let Some(code) = status_code {
                        self.config.is_retryable(code)
                    } else {
                        // 没有状态码的错误（如网络错误）默认可重试
                        true
                    };

                    // 检查是否还有重试次数
                    if !should_retry || attempts > self.config.max_retries {
                        return Err(RetryError {
                            attempts,
                            last_error,
                            last_status_code,
                        });
                    }

                    // 等待退避时间
                    let delay = self.backoff_delay(attempts - 1);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// 同步计算重试序列的所有退避时间（用于测试）
    pub fn compute_backoff_sequence(&self, jitter_factor: f64) -> Vec<Duration> {
        (0..self.config.max_retries)
            .map(|attempt| self.backoff_delay_with_jitter(attempt, jitter_factor))
            .collect()
    }
}

/// 生成 [0.0, 1.0) 范围内的随机抖动因子
fn rand_jitter_factor() -> f64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    // 使用简单的哈希方法生成伪随机数
    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64,
    );
    let hash = hasher.finish();

    (hash as f64) / (u64::MAX as f64)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert!(config.retryable_codes.contains(&429));
        assert!(config.retryable_codes.contains(&503));
    }

    #[test]
    fn test_retry_config_is_retryable() {
        let config = RetryConfig::default();

        // 可重试的状态码
        assert!(config.is_retryable(408));
        assert!(config.is_retryable(429));
        assert!(config.is_retryable(500));
        assert!(config.is_retryable(502));
        assert!(config.is_retryable(503));
        assert!(config.is_retryable(504));

        // 不可重试的状态码
        assert!(!config.is_retryable(200));
        assert!(!config.is_retryable(400));
        assert!(!config.is_retryable(401));
        assert!(!config.is_retryable(403));
        assert!(!config.is_retryable(404));
    }

    #[test]
    fn test_backoff_delay_no_jitter() {
        let config = RetryConfig::new(5, 1000, 30000);
        let retrier = Retrier::new(config);

        // 无抖动时的退避时间
        assert_eq!(
            retrier.backoff_delay_with_jitter(0, 0.0),
            Duration::from_millis(1000)
        );
        assert_eq!(
            retrier.backoff_delay_with_jitter(1, 0.0),
            Duration::from_millis(2000)
        );
        assert_eq!(
            retrier.backoff_delay_with_jitter(2, 0.0),
            Duration::from_millis(4000)
        );
        assert_eq!(
            retrier.backoff_delay_with_jitter(3, 0.0),
            Duration::from_millis(8000)
        );
        assert_eq!(
            retrier.backoff_delay_with_jitter(4, 0.0),
            Duration::from_millis(16000)
        );
    }

    #[test]
    fn test_backoff_delay_with_jitter() {
        let config = RetryConfig::new(5, 1000, 30000);
        let retrier = Retrier::new(config);

        // 50% 抖动
        assert_eq!(
            retrier.backoff_delay_with_jitter(0, 0.5),
            Duration::from_millis(1500)
        );
        assert_eq!(
            retrier.backoff_delay_with_jitter(1, 0.5),
            Duration::from_millis(2500)
        );
    }

    #[test]
    fn test_backoff_delay_max_cap() {
        let config = RetryConfig::new(10, 1000, 5000);
        let retrier = Retrier::new(config);

        // 应该被限制在 max_delay_ms
        assert_eq!(
            retrier.backoff_delay_with_jitter(5, 0.0),
            Duration::from_millis(5000)
        );
        assert_eq!(
            retrier.backoff_delay_with_jitter(10, 0.0),
            Duration::from_millis(5000)
        );
    }

    #[test]
    fn test_compute_backoff_sequence() {
        let config = RetryConfig::new(3, 1000, 30000);
        let retrier = Retrier::new(config);

        let sequence = retrier.compute_backoff_sequence(0.0);
        assert_eq!(sequence.len(), 3);
        assert_eq!(sequence[0], Duration::from_millis(1000));
        assert_eq!(sequence[1], Duration::from_millis(2000));
        assert_eq!(sequence[2], Duration::from_millis(4000));
    }

    #[tokio::test]
    async fn test_execute_success_first_try() {
        let retrier = Retrier::with_defaults();

        let result: Result<i32, RetryError> = retrier
            .execute(|| async { Ok::<_, (String, Option<u16>)>(42) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_execute_non_retryable_error() {
        let retrier = Retrier::with_defaults();

        let result: Result<i32, RetryError> = retrier
            .execute(|| async { Err::<i32, _>(("Bad Request".to_string(), Some(400))) })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.attempts, 1); // 只尝试一次
        assert_eq!(err.last_status_code, Some(400));
    }
}
