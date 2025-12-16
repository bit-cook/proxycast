//! 容错机制属性测试
//!
//! 使用 proptest 进行属性测试

use crate::resilience::{Retrier, RetryConfig};
use proptest::prelude::*;
use std::time::Duration;

/// 生成有效的重试配置（用于属性测试，使用较短的延迟）
fn arb_retry_config() -> impl Strategy<Value = RetryConfig> {
    (
        1u32..=5u32,    // max_retries (限制较小以加快测试)
        1u64..=10u64,   // base_delay_ms (使用毫秒级延迟)
        10u64..=100u64, // max_delay_ms
    )
        .prop_map(|(max_retries, base_delay_ms, max_delay_ms)| {
            // 确保 max_delay >= base_delay
            let max_delay_ms = max_delay_ms.max(base_delay_ms);
            RetryConfig::new(max_retries, base_delay_ms, max_delay_ms)
        })
}

proptest! {
    /// **Feature: enhancement-roadmap, Property 9: 重试次数限制**
    /// *对于任意* 请求，重试次数不应超过配置的最大值
    /// **Validates: Requirements 3.1 (验收标准 1)**
    #[test]
    fn prop_retry_count_limit(
        config in arb_retry_config()
    ) {
        let max_retries = config.max_retries;
        let retrier = Retrier::new(config);

        // 使用 tokio runtime 执行异步测试
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();

        let result = rt.block_on(async {
            let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
            let attempt_count_clone = attempt_count.clone();

            // 模拟总是失败的可重试操作
            let result: Result<(), _> = retrier
                .execute(|| {
                    let count = attempt_count_clone.clone();
                    async move {
                        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        // 返回可重试的错误 (503)
                        Err::<(), _>(("Service Unavailable".to_string(), Some(503u16)))
                    }
                })
                .await;

            (result, attempt_count.load(std::sync::atomic::Ordering::SeqCst))
        });

        let (result, actual_attempts) = result;

        // 验证：操作失败
        prop_assert!(result.is_err(), "持续失败的操作应该返回错误");

        // 验证：尝试次数 = 1 (初始) + max_retries (重试)
        let expected_attempts = 1 + max_retries;
        prop_assert_eq!(
            actual_attempts,
            expected_attempts,
            "尝试次数应为 {} (1 + {}), 但实际为 {}",
            expected_attempts,
            max_retries,
            actual_attempts
        );

        // 验证：错误中的尝试次数正确
        let err = result.unwrap_err();
        prop_assert_eq!(
            err.attempts,
            expected_attempts,
            "错误中的尝试次数应为 {}",
            expected_attempts
        );
    }

    /// **Feature: enhancement-roadmap, Property 9: 重试次数限制（成功提前终止）**
    /// *对于任意* 请求，如果在重试过程中成功，应立即返回而不继续重试
    /// **Validates: Requirements 3.1 (验收标准 1)**
    #[test]
    fn prop_retry_stops_on_success(
        config in arb_retry_config(),
        success_at in 1u32..=10u32
    ) {
        let max_retries = config.max_retries;
        // 确保 success_at 在有效范围内
        let success_at = success_at.min(max_retries + 1);

        let retrier = Retrier::new(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();

        let result = rt.block_on(async {
            let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
            let attempt_count_clone = attempt_count.clone();
            let success_at_clone = success_at;

            let result: Result<u32, _> = retrier
                .execute(|| {
                    let count = attempt_count_clone.clone();
                    async move {
                        let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        if current >= success_at_clone {
                            Ok(current)
                        } else {
                            Err(("Temporary failure".to_string(), Some(503u16)))
                        }
                    }
                })
                .await;

            (result, attempt_count.load(std::sync::atomic::Ordering::SeqCst))
        });

        let (result, actual_attempts) = result;

        // 验证：操作成功
        prop_assert!(result.is_ok(), "应该在第 {} 次尝试时成功", success_at);

        // 验证：尝试次数正好是 success_at
        prop_assert_eq!(
            actual_attempts,
            success_at,
            "尝试次数应为 {}，但实际为 {}",
            success_at,
            actual_attempts
        );
    }

    /// **Feature: enhancement-roadmap, Property 9: 重试次数限制（不可重试错误）**
    /// *对于任意* 不可重试的错误，应立即返回而不重试
    /// **Validates: Requirements 3.1 (验收标准 1)**
    #[test]
    fn prop_no_retry_on_non_retryable_error(
        config in arb_retry_config(),
        status_code in prop::sample::select(vec![400u16, 401, 403, 404, 405, 422])
    ) {
        let retrier = Retrier::new(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();

        let result = rt.block_on(async {
            let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
            let attempt_count_clone = attempt_count.clone();
            let status = status_code;

            let result: Result<(), _> = retrier
                .execute(|| {
                    let count = attempt_count_clone.clone();
                    async move {
                        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        Err::<(), _>(("Client error".to_string(), Some(status)))
                    }
                })
                .await;

            (result, attempt_count.load(std::sync::atomic::Ordering::SeqCst))
        });

        let (result, actual_attempts) = result;

        // 验证：操作失败
        prop_assert!(result.is_err(), "不可重试的错误应该返回错误");

        // 验证：只尝试一次
        prop_assert_eq!(
            actual_attempts,
            1,
            "不可重试的错误应该只尝试 1 次，但实际尝试了 {} 次",
            actual_attempts
        );
    }
}

proptest! {
    /// **Feature: enhancement-roadmap, Property 10: 退避时间递增**
    /// *对于任意* 重试序列，第 N+1 次重试的等待时间应大于等于第 N 次
    /// **Validates: Requirements 3.1 (验收标准 2)**
    #[test]
    fn prop_backoff_time_increasing(
        config in arb_retry_config(),
        jitter_factor in 0.0f64..1.0f64
    ) {
        let retrier = Retrier::new(config.clone());

        // 计算退避序列
        let sequence = retrier.compute_backoff_sequence(jitter_factor);

        // 验证：序列非空
        prop_assert!(!sequence.is_empty(), "退避序列不应为空");

        // 验证：每个退避时间都不超过最大值
        let max_delay = Duration::from_millis(config.max_delay_ms);
        for (i, delay) in sequence.iter().enumerate() {
            prop_assert!(
                *delay <= max_delay,
                "第 {} 次退避时间 {:?} 不应超过最大值 {:?}",
                i,
                delay,
                max_delay
            );
        }

        // 验证：在达到最大值之前，退避时间应该递增
        // 注意：由于抖动的存在，我们只验证无抖动情况下的递增性
        let sequence_no_jitter = retrier.compute_backoff_sequence(0.0);
        for i in 1..sequence_no_jitter.len() {
            prop_assert!(
                sequence_no_jitter[i] >= sequence_no_jitter[i - 1],
                "无抖动时，第 {} 次退避时间 {:?} 应 >= 第 {} 次 {:?}",
                i,
                sequence_no_jitter[i],
                i - 1,
                sequence_no_jitter[i - 1]
            );
        }
    }

    /// **Feature: enhancement-roadmap, Property 10: 退避时间递增（指数增长）**
    /// *对于任意* 重试配置，无抖动时退避时间应按 2^n 指数增长
    /// **Validates: Requirements 3.1 (验收标准 2)**
    #[test]
    fn prop_backoff_exponential_growth(
        max_retries in 1u32..=5u32,
        base_delay_ms in 100u64..=1000u64
    ) {
        // 使用足够大的 max_delay 以避免被截断
        let max_delay_ms = base_delay_ms * 2u64.pow(max_retries + 1);
        let config = RetryConfig::new(max_retries, base_delay_ms, max_delay_ms);
        let retrier = Retrier::new(config);

        // 无抖动的退避序列
        let sequence = retrier.compute_backoff_sequence(0.0);

        // 验证：每次退避时间是 base * 2^attempt
        for (attempt, delay) in sequence.iter().enumerate() {
            let expected = Duration::from_millis(base_delay_ms * 2u64.pow(attempt as u32));
            prop_assert_eq!(
                *delay,
                expected,
                "第 {} 次退避时间应为 {:?}，但实际为 {:?}",
                attempt,
                expected,
                delay
            );
        }
    }

    /// **Feature: enhancement-roadmap, Property 10: 退避时间递增（最大值限制）**
    /// *对于任意* 重试配置，退避时间不应超过配置的最大值
    /// **Validates: Requirements 3.1 (验收标准 2)**
    #[test]
    fn prop_backoff_max_cap(
        config in arb_retry_config(),
        attempt in 0u32..=20u32
    ) {
        let retrier = Retrier::new(config.clone());
        let max_delay = Duration::from_millis(config.max_delay_ms);

        // 测试任意尝试次数的退避时间
        let delay = retrier.backoff_delay_with_jitter(attempt, 0.99); // 使用最大抖动

        prop_assert!(
            delay <= max_delay,
            "第 {} 次退避时间 {:?} 不应超过最大值 {:?}",
            attempt,
            delay,
            max_delay
        );
    }

    /// **Feature: enhancement-roadmap, Property 10: 退避时间递增（抖动范围）**
    /// *对于任意* 重试配置，抖动应在 [0, base_delay) 范围内
    /// **Validates: Requirements 3.1 (验收标准 2)**
    #[test]
    fn prop_backoff_jitter_range(
        config in arb_retry_config(),
        attempt in 0u32..=5u32,
        jitter_factor in 0.0f64..1.0f64
    ) {
        let retrier = Retrier::new(config.clone());

        let delay_no_jitter = retrier.backoff_delay_with_jitter(attempt, 0.0);
        let delay_with_jitter = retrier.backoff_delay_with_jitter(attempt, jitter_factor);

        // 抖动应该使延迟增加，但增加量不超过 base_delay
        let max_jitter = Duration::from_millis(config.base_delay_ms);
        let actual_jitter = delay_with_jitter.saturating_sub(delay_no_jitter);

        // 如果没有被 max_delay 截断，抖动应该在范围内
        if delay_no_jitter < Duration::from_millis(config.max_delay_ms) {
            prop_assert!(
                actual_jitter <= max_jitter,
                "抖动 {:?} 不应超过 base_delay {:?}",
                actual_jitter,
                max_jitter
            );
        }
    }
}
