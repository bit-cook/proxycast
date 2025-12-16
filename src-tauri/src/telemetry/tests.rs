//! 遥测模块属性测试
//!
//! 使用 proptest 进行属性测试

use crate::telemetry::{
    LogRotationConfig, RequestLog, RequestLogger, RequestStatus, StatsAggregator, TimeRange,
};
use crate::ProviderType;
use chrono::{Duration, Utc};
use proptest::prelude::*;
use std::collections::HashSet;

/// 生成随机的 ProviderType
fn arb_provider_type() -> impl Strategy<Value = ProviderType> {
    prop_oneof![
        Just(ProviderType::Kiro),
        Just(ProviderType::Gemini),
        Just(ProviderType::Qwen),
        Just(ProviderType::OpenAI),
        Just(ProviderType::Claude),
    ]
}

/// 生成随机的 RequestStatus
fn arb_request_status() -> impl Strategy<Value = RequestStatus> {
    prop_oneof![
        Just(RequestStatus::Success),
        Just(RequestStatus::Failed),
        Just(RequestStatus::Timeout),
        Just(RequestStatus::Cancelled),
    ]
}

/// 生成随机的模型名称
fn arb_model_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("claude-sonnet-4".to_string()),
        Just("claude-opus-4".to_string()),
        Just("gemini-2.5-flash".to_string()),
        Just("gemini-2.5-pro".to_string()),
        Just("qwen3-coder-plus".to_string()),
        Just("gpt-4o".to_string()),
    ]
}

/// 生成随机的请求日志
fn arb_request_log() -> impl Strategy<Value = RequestLog> {
    (
        "[a-zA-Z0-9_-]{8,16}", // id
        arb_provider_type(),
        arb_model_name(),
        any::<bool>(), // is_streaming
        arb_request_status(),
        1u64..10000u64,                   // duration_ms
        prop::option::of(100u16..600u16), // http_status
        prop::option::of(1u32..10000u32), // input_tokens
        prop::option::of(1u32..5000u32),  // output_tokens
    )
        .prop_map(
            |(
                id,
                provider,
                model,
                is_streaming,
                status,
                duration_ms,
                http_status,
                input_tokens,
                output_tokens,
            )| {
                let mut log = RequestLog::new(id, provider, model, is_streaming);

                match status {
                    RequestStatus::Success => {
                        log.mark_success(duration_ms, http_status.unwrap_or(200));
                    }
                    RequestStatus::Failed => {
                        log.mark_failed(duration_ms, http_status, "Test error".to_string());
                    }
                    RequestStatus::Timeout => {
                        log.mark_timeout(duration_ms);
                    }
                    RequestStatus::Cancelled => {
                        log.mark_cancelled(duration_ms);
                    }
                    RequestStatus::Retrying => {
                        // 保持默认状态
                    }
                }

                log.set_tokens(input_tokens, output_tokens);
                log
            },
        )
}

/// 生成具有唯一 ID 的请求日志列表
fn arb_unique_request_logs(max_count: usize) -> impl Strategy<Value = Vec<RequestLog>> {
    prop::collection::vec(arb_request_log(), 1..=max_count).prop_map(|logs| {
        let mut seen = HashSet::new();
        logs.into_iter()
            .filter(|l| seen.insert(l.id.clone()))
            .collect()
    })
}

/// 创建测试用的日志记录器（禁用文件日志）
fn create_test_logger() -> RequestLogger {
    let config = LogRotationConfig {
        max_memory_logs: 1000,
        retention_days: 7,
        max_file_size: 10 * 1024 * 1024,
        enable_file_logging: false, // 测试时禁用文件日志
    };
    RequestLogger::new(config).expect("Failed to create test logger")
}

proptest! {
    /// **Feature: enhancement-roadmap, Property 13: 日志完整性**
    /// *对于任意* 已处理的请求，日志中应存在对应的记录
    /// **Validates: Requirements 5.1 (验收标准 1)**
    #[test]
    fn prop_log_completeness(
        logs in arb_unique_request_logs(50)
    ) {
        let logger = create_test_logger();
        let log_ids: Vec<String> = logs.iter().map(|l| l.id.clone()).collect();

        // 记录所有日志
        for log in logs {
            logger.record(log).expect("Failed to record log");
        }

        // 验证：所有记录的日志都应该能被检索到
        for id in &log_ids {
            let found = logger.get_by_id(id);
            prop_assert!(
                found.is_some(),
                "日志 {} 应该存在于记录器中",
                id
            );
        }

        // 验证：日志数量应该正确
        prop_assert_eq!(
            logger.len(),
            log_ids.len(),
            "日志记录器中的日志数量应该等于记录的数量"
        );
    }

    /// **Feature: enhancement-roadmap, Property 13: 日志完整性（内容一致性）**
    /// *对于任意* 请求日志，记录后检索的内容应与原始内容一致
    /// **Validates: Requirements 5.1 (验收标准 1)**
    #[test]
    fn prop_log_content_consistency(
        log in arb_request_log()
    ) {
        let logger = create_test_logger();
        let original_id = log.id.clone();
        let original_provider = log.provider;
        let original_model = log.model.clone();
        let original_status = log.status;
        let original_duration = log.duration_ms;

        // 记录日志
        logger.record(log).expect("Failed to record log");

        // 检索日志
        let retrieved = logger.get_by_id(&original_id).expect("Log should exist");

        // 验证内容一致性
        prop_assert_eq!(
            retrieved.id,
            original_id,
            "检索的日志 ID 应与原始一致"
        );
        prop_assert_eq!(
            retrieved.provider,
            original_provider,
            "检索的日志 Provider 应与原始一致"
        );
        prop_assert_eq!(
            retrieved.model,
            original_model,
            "检索的日志模型应与原始一致"
        );
        prop_assert_eq!(
            retrieved.status,
            original_status,
            "检索的日志状态应与原始一致"
        );
        prop_assert_eq!(
            retrieved.duration_ms,
            original_duration,
            "检索的日志持续时间应与原始一致"
        );
    }

    /// **Feature: enhancement-roadmap, Property 13: 日志完整性（Provider 过滤）**
    /// *对于任意* Provider 过滤，返回的日志应全部属于该 Provider
    /// **Validates: Requirements 5.1 (验收标准 2)**
    #[test]
    fn prop_log_provider_filter(
        logs in arb_unique_request_logs(30),
        filter_provider in arb_provider_type()
    ) {
        let logger = create_test_logger();

        // 记录所有日志
        for log in &logs {
            logger.record(log.clone()).expect("Failed to record log");
        }

        // 按 Provider 过滤
        let filtered = logger.get_by_provider(filter_provider);

        // 验证：所有返回的日志都应属于指定的 Provider
        for log in &filtered {
            prop_assert_eq!(
                log.provider,
                filter_provider,
                "过滤后的日志 Provider 应为 {:?}",
                filter_provider
            );
        }

        // 验证：返回的数量应等于原始日志中该 Provider 的数量
        let expected_count = logs.iter().filter(|l| l.provider == filter_provider).count();
        prop_assert_eq!(
            filtered.len(),
            expected_count,
            "过滤后的日志数量应正确"
        );
    }

    /// **Feature: enhancement-roadmap, Property 13: 日志完整性（状态过滤）**
    /// *对于任意* 状态过滤，返回的日志应全部具有该状态
    /// **Validates: Requirements 5.1 (验收标准 2)**
    #[test]
    fn prop_log_status_filter(
        logs in arb_unique_request_logs(30),
        filter_status in arb_request_status()
    ) {
        let logger = create_test_logger();

        // 记录所有日志
        for log in &logs {
            logger.record(log.clone()).expect("Failed to record log");
        }

        // 按状态过滤
        let filtered = logger.get_by_status(filter_status);

        // 验证：所有返回的日志都应具有指定的状态
        for log in &filtered {
            prop_assert_eq!(
                log.status,
                filter_status,
                "过滤后的日志状态应为 {:?}",
                filter_status
            );
        }

        // 验证：返回的数量应等于原始日志中该状态的数量
        let expected_count = logs.iter().filter(|l| l.status == filter_status).count();
        prop_assert_eq!(
            filtered.len(),
            expected_count,
            "过滤后的日志数量应正确"
        );
    }

    /// **Feature: enhancement-roadmap, Property 13: 日志完整性（统计准确性）**
    /// *对于任意* 日志集合，统计的总请求数应等于日志条目数
    /// **Validates: Requirements 5.1, 5.2 (验收标准 1)**
    #[test]
    fn prop_stats_accuracy(
        logs in arb_unique_request_logs(50)
    ) {
        let logger = create_test_logger();
        let total_count = logs.len();
        let success_count = logs.iter().filter(|l| l.status == RequestStatus::Success).count();
        let failed_count = logs.iter().filter(|l| l.status == RequestStatus::Failed).count();
        let timeout_count = logs.iter().filter(|l| l.status == RequestStatus::Timeout).count();

        // 记录所有日志
        for log in logs {
            logger.record(log).expect("Failed to record log");
        }

        // 获取统计摘要
        let summary = logger.summary(None);

        // 验证：总请求数应等于日志条目数
        prop_assert_eq!(
            summary.total_requests as usize,
            total_count,
            "统计的总请求数应等于日志条目数"
        );

        // 验证：成功请求数应正确
        prop_assert_eq!(
            summary.successful_requests as usize,
            success_count,
            "统计的成功请求数应正确"
        );

        // 验证：失败请求数应正确
        prop_assert_eq!(
            summary.failed_requests as usize,
            failed_count,
            "统计的失败请求数应正确"
        );

        // 验证：超时请求数应正确
        prop_assert_eq!(
            summary.timeout_requests as usize,
            timeout_count,
            "统计的超时请求数应正确"
        );

        // 验证：成功率计算正确
        if total_count > 0 {
            let expected_rate = success_count as f64 / total_count as f64;
            prop_assert!(
                (summary.success_rate - expected_rate).abs() < 0.001,
                "成功率应正确计算"
            );
        }
    }

    /// **Feature: enhancement-roadmap, Property 13: 日志完整性（内存轮转）**
    /// *对于任意* 超过内存限制的日志，应保留最新的日志
    /// **Validates: Requirements 5.1 (验收标准 4)**
    #[test]
    fn prop_memory_rotation(
        log_count in 10usize..100usize
    ) {
        let max_logs = 20usize;
        let config = LogRotationConfig {
            max_memory_logs: max_logs,
            retention_days: 7,
            max_file_size: 10 * 1024 * 1024,
            enable_file_logging: false,
        };
        let logger = RequestLogger::new(config).expect("Failed to create logger");

        // 生成并记录日志
        let mut all_ids: Vec<String> = Vec::new();
        for i in 0..log_count {
            let log = RequestLog::new(
                format!("log-{:04}", i),
                ProviderType::Kiro,
                "test-model".to_string(),
                false,
            );
            all_ids.push(log.id.clone());
            logger.record(log).expect("Failed to record log");
        }

        // 验证：日志数量不超过限制
        prop_assert!(
            logger.len() <= max_logs,
            "日志数量 {} 不应超过限制 {}",
            logger.len(),
            max_logs
        );

        // 验证：保留的是最新的日志
        if log_count > max_logs {
            let expected_oldest_index = log_count - max_logs;
            let expected_oldest_id = format!("log-{:04}", expected_oldest_index);

            // 最旧的保留日志应该是 expected_oldest_id
            let oldest_retained = logger.get_by_id(&expected_oldest_id);
            prop_assert!(
                oldest_retained.is_some(),
                "最旧的保留日志 {} 应该存在",
                expected_oldest_id
            );

            // 更早的日志应该已被删除
            if expected_oldest_index > 0 {
                let deleted_id = format!("log-{:04}", expected_oldest_index - 1);
                let deleted = logger.get_by_id(&deleted_id);
                prop_assert!(
                    deleted.is_none(),
                    "已轮转的日志 {} 不应存在",
                    deleted_id
                );
            }
        }
    }
}

// ========== 单元测试 ==========

#[test]
fn test_logger_basic_operations() {
    let logger = create_test_logger();

    // 创建并记录日志
    let mut log = RequestLog::new(
        "test-1".to_string(),
        ProviderType::Kiro,
        "claude-sonnet".to_string(),
        false,
    );
    log.mark_success(100, 200);

    logger.record(log).expect("Failed to record log");

    // 验证
    assert_eq!(logger.len(), 1);
    assert!(!logger.is_empty());

    let retrieved = logger.get_by_id("test-1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().status, RequestStatus::Success);
}

#[test]
fn test_logger_clear() {
    let logger = create_test_logger();

    // 记录一些日志
    for i in 0..5 {
        let log = RequestLog::new(
            format!("test-{}", i),
            ProviderType::Kiro,
            "model".to_string(),
            false,
        );
        logger.record(log).expect("Failed to record log");
    }

    assert_eq!(logger.len(), 5);

    // 清空
    logger.clear();

    assert_eq!(logger.len(), 0);
    assert!(logger.is_empty());
}

#[test]
fn test_stats_by_provider() {
    let logger = create_test_logger();

    // 记录不同 Provider 的日志
    for provider in [ProviderType::Kiro, ProviderType::Gemini, ProviderType::Kiro] {
        let mut log = RequestLog::new(
            uuid::Uuid::new_v4().to_string(),
            provider,
            "model".to_string(),
            false,
        );
        log.mark_success(100, 200);
        logger.record(log).expect("Failed to record log");
    }

    let stats = logger.stats_by_provider(None);

    assert!(stats.contains_key(&ProviderType::Kiro));
    assert!(stats.contains_key(&ProviderType::Gemini));
    assert_eq!(stats[&ProviderType::Kiro].summary.total_requests, 2);
    assert_eq!(stats[&ProviderType::Gemini].summary.total_requests, 1);
}

#[test]
fn test_stats_by_model() {
    let logger = create_test_logger();

    // 记录不同模型的日志
    for model in ["model-a", "model-b", "model-a"] {
        let mut log = RequestLog::new(
            uuid::Uuid::new_v4().to_string(),
            ProviderType::Kiro,
            model.to_string(),
            false,
        );
        log.mark_success(100, 200);
        logger.record(log).expect("Failed to record log");
    }

    let stats = logger.stats_by_model(None);

    assert!(stats.contains_key("model-a"));
    assert!(stats.contains_key("model-b"));
    assert_eq!(stats["model-a"].summary.total_requests, 2);
    assert_eq!(stats["model-b"].summary.total_requests, 1);
}

// ========== StatsAggregator 属性测试 ==========

/// 创建测试用的统计聚合器
fn create_test_aggregator() -> StatsAggregator {
    StatsAggregator::new(Duration::days(7), 10000)
}

proptest! {
    /// **Feature: enhancement-roadmap, Property 14: 统计准确性**
    /// *对于任意* 时间范围，统计的总请求数应等于该范围内日志条目数
    /// **Validates: Requirements 5.2 (验收标准 1)**
    #[test]
    fn prop_stats_aggregator_accuracy(
        logs in arb_unique_request_logs(50)
    ) {
        let aggregator = create_test_aggregator();
        let total_count = logs.len();
        let success_count = logs.iter().filter(|l| l.status == RequestStatus::Success).count();
        let failed_count = logs.iter().filter(|l| l.status == RequestStatus::Failed).count();
        let timeout_count = logs.iter().filter(|l| l.status == RequestStatus::Timeout).count();

        // 记录所有日志
        for log in logs {
            aggregator.record(log);
        }

        // 获取统计摘要
        let summary = aggregator.summary(None);

        // 验证：总请求数应等于日志条目数
        prop_assert_eq!(
            summary.total_requests as usize,
            total_count,
            "统计的总请求数应等于日志条目数"
        );

        // 验证：成功请求数应正确
        prop_assert_eq!(
            summary.successful_requests as usize,
            success_count,
            "统计的成功请求数应正确"
        );

        // 验证：失败请求数应正确
        prop_assert_eq!(
            summary.failed_requests as usize,
            failed_count,
            "统计的失败请求数应正确"
        );

        // 验证：超时请求数应正确
        prop_assert_eq!(
            summary.timeout_requests as usize,
            timeout_count,
            "统计的超时请求数应正确"
        );

        // 验证：成功率计算正确
        if total_count > 0 {
            let expected_rate = success_count as f64 / total_count as f64;
            prop_assert!(
                (summary.success_rate - expected_rate).abs() < 0.001,
                "成功率应正确计算"
            );
        }
    }

    /// **Feature: enhancement-roadmap, Property 14: 统计准确性（Provider 分组）**
    /// *对于任意* 日志集合，按 Provider 分组后各组的请求数之和应等于总请求数
    /// **Validates: Requirements 5.2 (验收标准 2)**
    #[test]
    fn prop_stats_aggregator_provider_grouping(
        logs in arb_unique_request_logs(50)
    ) {
        let aggregator = create_test_aggregator();
        let total_count = logs.len();

        // 计算每个 Provider 的预期数量
        let mut expected_by_provider: std::collections::HashMap<ProviderType, usize> = std::collections::HashMap::new();
        for log in &logs {
            *expected_by_provider.entry(log.provider).or_default() += 1;
        }

        // 记录所有日志
        for log in logs {
            aggregator.record(log);
        }

        // 按 Provider 分组统计
        let stats_by_provider = aggregator.by_provider(None);

        // 验证：各组请求数之和应等于总请求数
        let sum: u64 = stats_by_provider.values().map(|s| s.summary.total_requests).sum();
        prop_assert_eq!(
            sum as usize,
            total_count,
            "各 Provider 请求数之和应等于总请求数"
        );

        // 验证：每个 Provider 的请求数应正确
        for (provider, expected_count) in expected_by_provider {
            if let Some(stats) = stats_by_provider.get(&provider) {
                prop_assert_eq!(
                    stats.summary.total_requests as usize,
                    expected_count,
                    "Provider {:?} 的请求数应正确",
                    provider
                );
            } else {
                prop_assert!(false, "Provider {:?} 应存在于统计中", provider);
            }
        }
    }

    /// **Feature: enhancement-roadmap, Property 14: 统计准确性（模型分组）**
    /// *对于任意* 日志集合，按模型分组后各组的请求数之和应等于总请求数
    /// **Validates: Requirements 5.2 (验收标准 2)**
    #[test]
    fn prop_stats_aggregator_model_grouping(
        logs in arb_unique_request_logs(50)
    ) {
        let aggregator = create_test_aggregator();
        let total_count = logs.len();

        // 计算每个模型的预期数量
        let mut expected_by_model: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for log in &logs {
            *expected_by_model.entry(log.model.clone()).or_default() += 1;
        }

        // 记录所有日志
        for log in logs {
            aggregator.record(log);
        }

        // 按模型分组统计
        let stats_by_model = aggregator.by_model(None);

        // 验证：各组请求数之和应等于总请求数
        let sum: u64 = stats_by_model.values().map(|s| s.summary.total_requests).sum();
        prop_assert_eq!(
            sum as usize,
            total_count,
            "各模型请求数之和应等于总请求数"
        );

        // 验证：每个模型的请求数应正确
        for (model, expected_count) in expected_by_model {
            if let Some(stats) = stats_by_model.get(&model) {
                prop_assert_eq!(
                    stats.summary.total_requests as usize,
                    expected_count,
                    "模型 {} 的请求数应正确",
                    model
                );
            } else {
                prop_assert!(false, "模型 {} 应存在于统计中", model);
            }
        }
    }

    /// **Feature: enhancement-roadmap, Property 14: 统计准确性（状态分组）**
    /// *对于任意* 日志集合，按状态分组后各组的请求数之和应等于总请求数
    /// **Validates: Requirements 5.2 (验收标准 1)**
    #[test]
    fn prop_stats_aggregator_status_grouping(
        logs in arb_unique_request_logs(50)
    ) {
        let aggregator = create_test_aggregator();
        let total_count = logs.len();

        // 计算每个状态的预期数量
        let mut expected_by_status: std::collections::HashMap<RequestStatus, usize> = std::collections::HashMap::new();
        for log in &logs {
            *expected_by_status.entry(log.status).or_default() += 1;
        }

        // 记录所有日志
        for log in logs {
            aggregator.record(log);
        }

        // 按状态分组统计
        let stats_by_status = aggregator.by_status(None);

        // 验证：各组请求数之和应等于总请求数
        let sum: u64 = stats_by_status.values().sum();
        prop_assert_eq!(
            sum as usize,
            total_count,
            "各状态请求数之和应等于总请求数"
        );

        // 验证：每个状态的请求数应正确
        for (status, expected_count) in expected_by_status {
            if let Some(&count) = stats_by_status.get(&status) {
                prop_assert_eq!(
                    count as usize,
                    expected_count,
                    "状态 {:?} 的请求数应正确",
                    status
                );
            } else {
                prop_assert!(false, "状态 {:?} 应存在于统计中", status);
            }
        }
    }
}

// ========== StatsAggregator 单元测试 ==========

#[test]
fn test_stats_aggregator_basic_operations() {
    let aggregator = create_test_aggregator();

    // 创建并记录日志
    let mut log = RequestLog::new(
        "test-1".to_string(),
        ProviderType::Kiro,
        "claude-sonnet".to_string(),
        false,
    );
    log.mark_success(100, 200);

    aggregator.record(log);

    // 验证
    assert_eq!(aggregator.len(), 1);
    assert!(!aggregator.is_empty());

    let summary = aggregator.summary(None);
    assert_eq!(summary.total_requests, 1);
    assert_eq!(summary.successful_requests, 1);
}

#[test]
fn test_stats_aggregator_clear() {
    let aggregator = create_test_aggregator();

    // 记录一些日志
    for i in 0..5 {
        let log = RequestLog::new(
            format!("test-{}", i),
            ProviderType::Kiro,
            "model".to_string(),
            false,
        );
        aggregator.record(log);
    }

    assert_eq!(aggregator.len(), 5);

    // 清空
    aggregator.clear();

    assert_eq!(aggregator.len(), 0);
    assert!(aggregator.is_empty());
}

#[test]
fn test_stats_aggregator_by_provider() {
    let aggregator = create_test_aggregator();

    // 记录不同 Provider 的日志
    for provider in [ProviderType::Kiro, ProviderType::Gemini, ProviderType::Kiro] {
        let mut log = RequestLog::new(
            uuid::Uuid::new_v4().to_string(),
            provider,
            "model".to_string(),
            false,
        );
        log.mark_success(100, 200);
        aggregator.record(log);
    }

    let stats = aggregator.by_provider(None);

    assert!(stats.contains_key(&ProviderType::Kiro));
    assert!(stats.contains_key(&ProviderType::Gemini));
    assert_eq!(stats[&ProviderType::Kiro].summary.total_requests, 2);
    assert_eq!(stats[&ProviderType::Gemini].summary.total_requests, 1);
}

#[test]
fn test_stats_aggregator_by_model() {
    let aggregator = create_test_aggregator();

    // 记录不同模型的日志
    for model in ["model-a", "model-b", "model-a"] {
        let mut log = RequestLog::new(
            uuid::Uuid::new_v4().to_string(),
            ProviderType::Kiro,
            model.to_string(),
            false,
        );
        log.mark_success(100, 200);
        aggregator.record(log);
    }

    let stats = aggregator.by_model(None);

    assert!(stats.contains_key("model-a"));
    assert!(stats.contains_key("model-b"));
    assert_eq!(stats["model-a"].summary.total_requests, 2);
    assert_eq!(stats["model-b"].summary.total_requests, 1);
}

#[test]
fn test_stats_aggregator_time_range() {
    let aggregator = create_test_aggregator();

    // 记录日志
    for i in 0..5 {
        let mut log = RequestLog::new(
            format!("test-{}", i),
            ProviderType::Kiro,
            "model".to_string(),
            false,
        );
        log.mark_success(100, 200);
        aggregator.record(log);
    }

    // 使用时间范围查询
    let now = Utc::now();
    let range = TimeRange::new(now - Duration::hours(1), now + Duration::seconds(1));
    let summary = aggregator.summary(Some(range));

    assert_eq!(summary.total_requests, 5);
}

#[test]
fn test_stats_aggregator_max_logs_limit() {
    let aggregator = StatsAggregator::new(Duration::days(7), 10);

    // 记录超过限制的日志
    for i in 0..20 {
        let log = RequestLog::new(
            format!("test-{}", i),
            ProviderType::Kiro,
            "model".to_string(),
            false,
        );
        aggregator.record(log);
    }

    // 验证日志数量不超过限制
    assert_eq!(aggregator.len(), 10);
}
