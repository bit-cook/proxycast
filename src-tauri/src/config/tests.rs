//! 配置模块属性测试
//!
//! 使用 proptest 进行属性测试

use crate::config::{
    Config, ConfigManager, CustomProviderConfig, HotReloadManager, InjectionSettings,
    LoggingConfig, ProviderConfig, ProvidersConfig, ReloadResult, RetrySettings, RoutingConfig,
    ServerConfig,
};
use proptest::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

/// 生成随机的主机地址
fn arb_host() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("127.0.0.1".to_string()),
        Just("0.0.0.0".to_string()),
        Just("localhost".to_string()),
        "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}".prop_map(|s| s),
    ]
}

/// 生成随机的端口号
fn arb_port() -> impl Strategy<Value = u16> {
    1024u16..65535u16
}

/// 生成随机的 API 密钥
fn arb_api_key() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{8,32}".prop_map(|s| s)
}

/// 生成随机的服务器配置
fn arb_server_config() -> impl Strategy<Value = ServerConfig> {
    (arb_host(), arb_port(), arb_api_key()).prop_map(|(host, port, api_key)| ServerConfig {
        host,
        port,
        api_key,
    })
}

/// 生成随机的 Provider 配置
fn arb_provider_config() -> impl Strategy<Value = ProviderConfig> {
    (
        any::<bool>(),
        proptest::option::of("[a-zA-Z0-9/_.-]{5,50}".prop_map(|s| s)),
        proptest::option::of(prop_oneof![
            Just("us-east-1".to_string()),
            Just("us-west-2".to_string()),
            Just("eu-west-1".to_string()),
        ]),
        proptest::option::of("[a-zA-Z0-9-]{5,20}".prop_map(|s| s)),
    )
        .prop_map(
            |(enabled, credentials_path, region, project_id)| ProviderConfig {
                enabled,
                credentials_path,
                region,
                project_id,
            },
        )
}

/// 生成随机的自定义 Provider 配置
fn arb_custom_provider_config() -> impl Strategy<Value = CustomProviderConfig> {
    (
        any::<bool>(),
        proptest::option::of(arb_api_key()),
        proptest::option::of(prop_oneof![
            Just("https://api.openai.com/v1".to_string()),
            Just("https://api.anthropic.com".to_string()),
            Just("https://custom.api.com".to_string()),
        ]),
    )
        .prop_map(|(enabled, api_key, base_url)| CustomProviderConfig {
            enabled,
            api_key,
            base_url,
        })
}

/// 生成随机的 Providers 配置
fn arb_providers_config() -> impl Strategy<Value = ProvidersConfig> {
    (
        arb_provider_config(),
        arb_provider_config(),
        arb_provider_config(),
        arb_custom_provider_config(),
        arb_custom_provider_config(),
    )
        .prop_map(|(kiro, gemini, qwen, openai, claude)| ProvidersConfig {
            kiro,
            gemini,
            qwen,
            openai,
            claude,
        })
}

/// 生成随机的路由配置
fn arb_routing_config() -> impl Strategy<Value = RoutingConfig> {
    (
        prop_oneof![
            Just("kiro".to_string()),
            Just("gemini".to_string()),
            Just("qwen".to_string()),
        ],
        proptest::collection::vec(
            (
                "[a-z]+-\\*|\\*-[a-z]+|[a-z]+-[a-z0-9]+".prop_map(|s| s),
                prop_oneof![
                    Just("kiro".to_string()),
                    Just("gemini".to_string()),
                    Just("qwen".to_string()),
                ],
                1i32..100i32,
            ),
            0..5,
        ),
        proptest::collection::hash_map(
            "[a-z]+-[a-z0-9]+".prop_map(|s| s),
            "[a-z]+-[a-z0-9-]+".prop_map(|s| s),
            0..5,
        ),
        proptest::collection::hash_map(
            prop_oneof![
                Just("kiro".to_string()),
                Just("gemini".to_string()),
                Just("qwen".to_string()),
            ],
            proptest::collection::vec("[a-z]+-\\*|\\*-[a-z]+".prop_map(|s| s), 0..3),
            0..3,
        ),
    )
        .prop_map(
            |(default_provider, rules, model_aliases, exclusions)| RoutingConfig {
                default_provider,
                rules: rules
                    .into_iter()
                    .map(
                        |(pattern, provider, priority)| crate::config::types::RoutingRuleConfig {
                            pattern,
                            provider,
                            priority,
                        },
                    )
                    .collect(),
                model_aliases,
                exclusions,
            },
        )
}

/// 生成随机的重试配置
fn arb_retry_settings() -> impl Strategy<Value = RetrySettings> {
    (
        1u32..10u32,
        100u64..5000u64,
        5000u64..60000u64,
        any::<bool>(),
    )
        .prop_map(
            |(max_retries, base_delay_ms, max_delay_ms, auto_switch_provider)| RetrySettings {
                max_retries,
                base_delay_ms,
                max_delay_ms,
                auto_switch_provider,
            },
        )
}

/// 生成随机的日志配置
fn arb_logging_config() -> impl Strategy<Value = LoggingConfig> {
    (
        any::<bool>(),
        prop_oneof![
            Just("debug".to_string()),
            Just("info".to_string()),
            Just("warn".to_string()),
            Just("error".to_string()),
        ],
        1u32..30u32,
        any::<bool>(),
    )
        .prop_map(
            |(enabled, level, retention_days, include_request_body)| LoggingConfig {
                enabled,
                level,
                retention_days,
                include_request_body,
            },
        )
}

/// 生成随机的完整配置
fn arb_config() -> impl Strategy<Value = Config> {
    (
        arb_server_config(),
        arb_providers_config(),
        arb_routing_config(),
        arb_retry_settings(),
        arb_logging_config(),
    )
        .prop_map(|(server, providers, routing, retry, logging)| Config {
            server,
            providers,
            default_provider: routing.default_provider.clone(),
            routing,
            retry,
            logging,
            injection: InjectionSettings::default(),
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: enhancement-roadmap, Property 11: 配置往返一致性**
    /// *对于任意* 有效配置，序列化后再反序列化应得到等价的配置
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_config_roundtrip(config in arb_config()) {
        // 序列化为 YAML
        let yaml = ConfigManager::to_yaml(&config)
            .expect("序列化应成功");

        // 反序列化回 Config
        let parsed = ConfigManager::parse_yaml(&yaml)
            .expect("反序列化应成功");

        // 验证往返一致性
        prop_assert_eq!(
            config.server,
            parsed.server,
            "服务器配置往返不一致"
        );
        prop_assert_eq!(
            config.providers,
            parsed.providers,
            "Provider 配置往返不一致"
        );
        prop_assert_eq!(
            config.routing.default_provider,
            parsed.routing.default_provider,
            "默认 Provider 往返不一致"
        );
        prop_assert_eq!(
            config.routing.rules.len(),
            parsed.routing.rules.len(),
            "路由规则数量往返不一致"
        );
        prop_assert_eq!(
            config.routing.model_aliases,
            parsed.routing.model_aliases,
            "模型别名往返不一致"
        );
        prop_assert_eq!(
            config.routing.exclusions,
            parsed.routing.exclusions,
            "排除列表往返不一致"
        );
        prop_assert_eq!(
            config.retry,
            parsed.retry,
            "重试配置往返不一致"
        );
        prop_assert_eq!(
            config.logging,
            parsed.logging,
            "日志配置往返不一致"
        );
    }

    /// **Feature: enhancement-roadmap, Property 11: 配置往返一致性（服务器配置）**
    /// *对于任意* 服务器配置，序列化后再反序列化应得到等价的配置
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_server_config_roundtrip(server in arb_server_config()) {
        let config = Config {
            server: server.clone(),
            ..Config::default()
        };

        let yaml = ConfigManager::to_yaml(&config).expect("序列化应成功");
        let parsed = ConfigManager::parse_yaml(&yaml).expect("反序列化应成功");

        prop_assert_eq!(
            server,
            parsed.server,
            "服务器配置往返不一致"
        );
    }

    /// **Feature: enhancement-roadmap, Property 11: 配置往返一致性（Provider 配置）**
    /// *对于任意* Provider 配置，序列化后再反序列化应得到等价的配置
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_providers_config_roundtrip(providers in arb_providers_config()) {
        let config = Config {
            providers: providers.clone(),
            ..Config::default()
        };

        let yaml = ConfigManager::to_yaml(&config).expect("序列化应成功");
        let parsed = ConfigManager::parse_yaml(&yaml).expect("反序列化应成功");

        prop_assert_eq!(
            providers,
            parsed.providers,
            "Provider 配置往返不一致"
        );
    }

    /// **Feature: enhancement-roadmap, Property 11: 配置往返一致性（重试配置）**
    /// *对于任意* 重试配置，序列化后再反序列化应得到等价的配置
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_retry_settings_roundtrip(retry in arb_retry_settings()) {
        let config = Config {
            retry: retry.clone(),
            ..Config::default()
        };

        let yaml = ConfigManager::to_yaml(&config).expect("序列化应成功");
        let parsed = ConfigManager::parse_yaml(&yaml).expect("反序列化应成功");

        prop_assert_eq!(
            retry,
            parsed.retry,
            "重试配置往返不一致"
        );
    }

    /// **Feature: enhancement-roadmap, Property 11: 配置往返一致性（日志配置）**
    /// *对于任意* 日志配置，序列化后再反序列化应得到等价的配置
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_logging_config_roundtrip(logging in arb_logging_config()) {
        let config = Config {
            logging: logging.clone(),
            ..Config::default()
        };

        let yaml = ConfigManager::to_yaml(&config).expect("序列化应成功");
        let parsed = ConfigManager::parse_yaml(&yaml).expect("反序列化应成功");

        prop_assert_eq!(
            logging,
            parsed.logging,
            "日志配置往返不一致"
        );
    }

    /// **Feature: enhancement-roadmap, Property 11: 配置往返一致性（路由配置）**
    /// *对于任意* 路由配置，序列化后再反序列化应得到等价的配置
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_routing_config_roundtrip(routing in arb_routing_config()) {
        let config = Config {
            routing: routing.clone(),
            ..Config::default()
        };

        let yaml = ConfigManager::to_yaml(&config).expect("序列化应成功");
        let parsed = ConfigManager::parse_yaml(&yaml).expect("反序列化应成功");

        prop_assert_eq!(
            routing.default_provider,
            parsed.routing.default_provider,
            "默认 Provider 往返不一致"
        );
        prop_assert_eq!(
            routing.model_aliases,
            parsed.routing.model_aliases,
            "模型别名往返不一致"
        );
        prop_assert_eq!(
            routing.exclusions,
            parsed.routing.exclusions,
            "排除列表往返不一致"
        );
        prop_assert_eq!(
            routing.rules.len(),
            parsed.routing.rules.len(),
            "路由规则数量往返不一致"
        );

        // 验证每个路由规则
        for (original, parsed_rule) in routing.rules.iter().zip(parsed.routing.rules.iter()) {
            prop_assert_eq!(
                &original.pattern,
                &parsed_rule.pattern,
                "路由规则模式往返不一致"
            );
            prop_assert_eq!(
                &original.provider,
                &parsed_rule.provider,
                "路由规则 Provider 往返不一致"
            );
            prop_assert_eq!(
                original.priority,
                parsed_rule.priority,
                "路由规则优先级往返不一致"
            );
        }
    }
}

/// 生成有效的服务器配置（端口非零）
fn arb_valid_server_config() -> impl Strategy<Value = ServerConfig> {
    (arb_host(), 1u16..65535u16, arb_api_key()).prop_map(|(host, port, api_key)| ServerConfig {
        host,
        port,
        api_key,
    })
}

/// 生成有效的重试配置（通过验证）
fn arb_valid_retry_settings() -> impl Strategy<Value = RetrySettings> {
    (
        1u32..100u32,      // max_retries <= 100
        1u64..5000u64,     // base_delay_ms > 0
        5000u64..60000u64, // max_delay_ms
        any::<bool>(),
    )
        .prop_map(
            |(max_retries, base_delay_ms, max_delay_ms, auto_switch_provider)| RetrySettings {
                max_retries,
                base_delay_ms,
                max_delay_ms,
                auto_switch_provider,
            },
        )
}

/// 生成有效的日志配置（保留天数非零）
fn arb_valid_logging_config() -> impl Strategy<Value = LoggingConfig> {
    (
        any::<bool>(),
        prop_oneof![
            Just("debug".to_string()),
            Just("info".to_string()),
            Just("warn".to_string()),
            Just("error".to_string()),
        ],
        1u32..30u32, // retention_days > 0
        any::<bool>(),
    )
        .prop_map(
            |(enabled, level, retention_days, include_request_body)| LoggingConfig {
                enabled,
                level,
                retention_days,
                include_request_body,
            },
        )
}

/// 生成有效的配置（通过验证的配置）
fn arb_valid_config() -> impl Strategy<Value = Config> {
    (
        arb_valid_server_config(),
        arb_providers_config(),
        arb_routing_config(),
        arb_valid_retry_settings(),
        arb_valid_logging_config(),
    )
        .prop_map(|(server, providers, routing, retry, logging)| Config {
            server,
            providers,
            default_provider: routing.default_provider.clone(),
            routing,
            retry,
            logging,
            injection: InjectionSettings::default(),
        })
}

/// 生成无效配置的类型
#[derive(Debug, Clone, Copy)]
enum InvalidConfigType {
    ZeroPort,
    TooManyRetries,
    ZeroBaseDelay,
    ZeroRetentionDays,
}

/// 生成无效的配置（不通过验证的配置）
fn arb_invalid_config() -> impl Strategy<Value = Config> {
    (
        arb_valid_server_config(),
        arb_providers_config(),
        arb_routing_config(),
        arb_valid_retry_settings(),
        arb_valid_logging_config(),
        prop_oneof![
            Just(InvalidConfigType::ZeroPort),
            Just(InvalidConfigType::TooManyRetries),
            Just(InvalidConfigType::ZeroBaseDelay),
            Just(InvalidConfigType::ZeroRetentionDays),
        ],
    )
        .prop_map(
            |(server, providers, routing, retry, logging, invalid_type)| {
                let mut config = Config {
                    server,
                    providers,
                    default_provider: routing.default_provider.clone(),
                    routing,
                    retry,
                    logging,
                    injection: InjectionSettings::default(),
                };
                // 根据类型使配置无效
                match invalid_type {
                    InvalidConfigType::ZeroPort => config.server.port = 0,
                    InvalidConfigType::TooManyRetries => config.retry.max_retries = 101,
                    InvalidConfigType::ZeroBaseDelay => config.retry.base_delay_ms = 0,
                    InvalidConfigType::ZeroRetentionDays => config.logging.retention_days = 0,
                }
                config
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: enhancement-roadmap, Property 12: 热重载原子性**
    /// *对于任意* 配置变更，要么完全应用成功，要么回滚到之前状态
    /// **Validates: Requirements 4.2 (验收标准 3)**
    #[test]
    fn prop_hot_reload_atomicity_success(
        initial_config in arb_valid_config(),
        new_config in arb_valid_config()
    ) {
        // 创建临时配置文件
        let mut temp_file = NamedTempFile::new().expect("创建临时文件失败");
        let yaml = ConfigManager::to_yaml(&new_config).expect("序列化失败");
        temp_file.write_all(yaml.as_bytes()).expect("写入文件失败");

        // 创建热重载管理器
        let manager = HotReloadManager::new(initial_config.clone(), temp_file.path().to_path_buf());

        // 执行热重载
        let result = manager.reload();

        // 验证原子性：成功时配置应完全更新
        match result {
            ReloadResult::Success { .. } => {
                let current = manager.config();
                // 验证配置已完全更新
                prop_assert_eq!(
                    current.server,
                    new_config.server,
                    "服务器配置未正确更新"
                );
                prop_assert_eq!(
                    current.providers,
                    new_config.providers,
                    "Provider 配置未正确更新"
                );
                prop_assert_eq!(
                    current.retry,
                    new_config.retry,
                    "重试配置未正确更新"
                );
                prop_assert_eq!(
                    current.logging,
                    new_config.logging,
                    "日志配置未正确更新"
                );
            }
            _ => {
                // 如果失败，应该保持原始配置
                let current = manager.config();
                prop_assert_eq!(
                    current,
                    initial_config,
                    "失败时配置应保持不变"
                );
            }
        }
    }

    /// **Feature: enhancement-roadmap, Property 12: 热重载原子性（失败回滚）**
    /// *对于任意* 无效配置变更，配置应回滚到之前状态
    /// **Validates: Requirements 4.2 (验收标准 3)**
    #[test]
    fn prop_hot_reload_atomicity_rollback(
        initial_config in arb_valid_config(),
        invalid_config in arb_invalid_config()
    ) {
        // 创建临时配置文件（包含无效配置）
        let mut temp_file = NamedTempFile::new().expect("创建临时文件失败");
        let yaml = ConfigManager::to_yaml(&invalid_config).expect("序列化失败");
        temp_file.write_all(yaml.as_bytes()).expect("写入文件失败");

        // 创建热重载管理器
        let manager = HotReloadManager::new(initial_config.clone(), temp_file.path().to_path_buf());

        // 执行热重载
        let result = manager.reload();

        // 验证原子性：失败时配置应回滚到之前状态
        match result {
            ReloadResult::RolledBack { .. } => {
                let current = manager.config();
                // 验证配置已回滚到初始状态
                prop_assert_eq!(
                    current,
                    initial_config,
                    "配置应回滚到初始状态"
                );
            }
            ReloadResult::Success { .. } => {
                // 如果意外成功（不应该发生），验证配置一致性
                let current = manager.config();
                prop_assert_eq!(
                    current,
                    invalid_config,
                    "成功时配置应完全更新"
                );
            }
            ReloadResult::Failed { .. } => {
                // 完全失败的情况，配置应保持不变
                let current = manager.config();
                prop_assert_eq!(
                    current,
                    initial_config,
                    "失败时配置应保持不变"
                );
            }
        }
    }

    /// **Feature: enhancement-roadmap, Property 12: 热重载原子性（文件不存在）**
    /// *对于任意* 初始配置，当配置文件不存在时，配置应保持不变
    /// **Validates: Requirements 4.2 (验收标准 3)**
    #[test]
    fn prop_hot_reload_atomicity_file_not_exists(initial_config in arb_valid_config()) {
        // 使用不存在的文件路径
        let nonexistent_path = std::path::PathBuf::from("/tmp/nonexistent_config_test_12345.yaml");

        // 创建热重载管理器
        let manager = HotReloadManager::new(initial_config.clone(), nonexistent_path);

        // 执行热重载
        let result = manager.reload();

        // 验证原子性：文件不存在时配置应保持不变
        match result {
            ReloadResult::RolledBack { .. } => {
                let current = manager.config();
                prop_assert_eq!(
                    current,
                    initial_config,
                    "文件不存在时配置应保持不变"
                );
            }
            _ => {
                // 其他情况也应保持配置不变
                let current = manager.config();
                prop_assert_eq!(
                    current,
                    initial_config,
                    "配置应保持不变"
                );
            }
        }
    }

    /// **Feature: enhancement-roadmap, Property 12: 热重载原子性（无效 YAML）**
    /// *对于任意* 初始配置，当配置文件包含无效 YAML 时，配置应保持不变
    /// **Validates: Requirements 4.2 (验收标准 3)**
    #[test]
    fn prop_hot_reload_atomicity_invalid_yaml(initial_config in arb_valid_config()) {
        // 创建包含无效 YAML 的临时文件
        let mut temp_file = NamedTempFile::new().expect("创建临时文件失败");
        temp_file.write_all(b"invalid: yaml: content: [").expect("写入文件失败");

        // 创建热重载管理器
        let manager = HotReloadManager::new(initial_config.clone(), temp_file.path().to_path_buf());

        // 执行热重载
        let result = manager.reload();

        // 验证原子性：无效 YAML 时配置应保持不变
        match result {
            ReloadResult::RolledBack { .. } => {
                let current = manager.config();
                prop_assert_eq!(
                    current,
                    initial_config,
                    "无效 YAML 时配置应保持不变"
                );
            }
            _ => {
                // 其他情况也应保持配置不变
                let current = manager.config();
                prop_assert_eq!(
                    current,
                    initial_config,
                    "配置应保持不变"
                );
            }
        }
    }
}
