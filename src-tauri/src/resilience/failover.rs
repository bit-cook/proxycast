//! 故障转移实现
//!
//! 提供 Provider 故障转移和自动切换功能

use crate::ProviderType;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 配额超限相关的 HTTP 状态码
pub const QUOTA_EXCEEDED_STATUS_CODES: &[u16] = &[429];

/// 配额超限相关的错误消息关键词
pub const QUOTA_EXCEEDED_KEYWORDS: &[&str] = &[
    "quota",
    "rate limit",
    "rate_limit",
    "too many requests",
    "exceeded",
    "limit exceeded",
    "throttl",
];

/// 故障转移配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FailoverConfig {
    /// 是否启用自动切换
    pub auto_switch: bool,
    /// 是否在配额超限时切换
    pub switch_on_quota: bool,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            auto_switch: true,
            switch_on_quota: true,
        }
    }
}

impl FailoverConfig {
    /// 创建新的故障转移配置
    pub fn new(auto_switch: bool, switch_on_quota: bool) -> Self {
        Self {
            auto_switch,
            switch_on_quota,
        }
    }

    /// 禁用自动切换的配置
    pub fn disabled() -> Self {
        Self {
            auto_switch: false,
            switch_on_quota: false,
        }
    }
}

/// 故障类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureType {
    /// 配额超限
    QuotaExceeded,
    /// 认证失败
    AuthenticationFailed,
    /// 服务不可用
    ServiceUnavailable,
    /// 其他错误
    Other,
}

impl FailureType {
    /// 从状态码和错误消息检测故障类型
    pub fn detect(status_code: Option<u16>, error_message: &str) -> Self {
        let error_lower = error_message.to_lowercase();

        // 检查配额超限
        if let Some(code) = status_code {
            if QUOTA_EXCEEDED_STATUS_CODES.contains(&code) {
                return FailureType::QuotaExceeded;
            }
        }

        // 检查错误消息中的配额关键词
        for keyword in QUOTA_EXCEEDED_KEYWORDS {
            if error_lower.contains(keyword) {
                return FailureType::QuotaExceeded;
            }
        }

        // 检查认证失败
        if let Some(code) = status_code {
            if code == 401 || code == 403 {
                return FailureType::AuthenticationFailed;
            }
        }

        // 检查服务不可用
        if let Some(code) = status_code {
            if code == 502 || code == 503 || code == 504 {
                return FailureType::ServiceUnavailable;
            }
        }

        FailureType::Other
    }

    /// 是否为配额超限
    pub fn is_quota_exceeded(&self) -> bool {
        matches!(self, FailureType::QuotaExceeded)
    }
}

/// 故障转移结果
#[derive(Debug, Clone)]
pub struct FailoverResult {
    /// 是否成功切换
    pub switched: bool,
    /// 新的 Provider（如果切换成功）
    pub new_provider: Option<ProviderType>,
    /// 故障类型
    pub failure_type: FailureType,
    /// 消息
    pub message: String,
}

impl FailoverResult {
    /// 创建成功切换的结果
    pub fn switched(new_provider: ProviderType, failure_type: FailureType) -> Self {
        Self {
            switched: true,
            new_provider: Some(new_provider),
            failure_type,
            message: format!("已切换到 Provider: {}", new_provider),
        }
    }

    /// 创建未切换的结果
    pub fn not_switched(failure_type: FailureType, message: &str) -> Self {
        Self {
            switched: false,
            new_provider: None,
            failure_type,
            message: message.to_string(),
        }
    }
}

/// 故障转移器
#[derive(Debug, Clone)]
pub struct Failover {
    config: FailoverConfig,
}

impl Failover {
    /// 创建新的故障转移器
    pub fn new(config: FailoverConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建故障转移器
    pub fn with_defaults() -> Self {
        Self::new(FailoverConfig::default())
    }

    /// 获取配置
    pub fn config(&self) -> &FailoverConfig {
        &self.config
    }

    /// 处理 Provider 失败
    ///
    /// 根据错误类型和配置决定是否切换 Provider
    ///
    /// # Arguments
    /// * `failed_provider` - 失败的 Provider
    /// * `status_code` - HTTP 状态码（如果有）
    /// * `error_message` - 错误消息
    /// * `available_providers` - 可用的 Provider 列表
    ///
    /// # Returns
    /// 故障转移结果，包含是否切换和新的 Provider
    pub fn handle_failure(
        &self,
        failed_provider: ProviderType,
        status_code: Option<u16>,
        error_message: &str,
        available_providers: &[ProviderType],
    ) -> FailoverResult {
        // 检测故障类型
        let failure_type = FailureType::detect(status_code, error_message);

        // 检查是否启用自动切换
        if !self.config.auto_switch {
            return FailoverResult::not_switched(failure_type, "自动切换已禁用");
        }

        // 检查是否应该在此类故障时切换
        let should_switch = match &failure_type {
            FailureType::QuotaExceeded => self.config.switch_on_quota,
            FailureType::ServiceUnavailable => true,
            FailureType::AuthenticationFailed => false, // 认证失败通常不应切换
            FailureType::Other => false,
        };

        if !should_switch {
            return FailoverResult::not_switched(
                failure_type,
                &format!("不在 {:?} 故障时切换", failure_type),
            );
        }

        // 选择替代 Provider
        match self.select_alternative(failed_provider, available_providers) {
            Some(new_provider) => FailoverResult::switched(new_provider, failure_type),
            None => FailoverResult::not_switched(failure_type, "没有可用的替代 Provider"),
        }
    }

    /// 选择替代 Provider
    ///
    /// 从可用 Provider 列表中选择一个不同于失败 Provider 的替代
    pub fn select_alternative(
        &self,
        failed_provider: ProviderType,
        available_providers: &[ProviderType],
    ) -> Option<ProviderType> {
        // 过滤掉失败的 Provider，选择第一个可用的
        available_providers
            .iter()
            .find(|&&p| p != failed_provider)
            .copied()
    }

    /// 选择替代 Provider（排除多个已失败的 Provider）
    ///
    /// 从可用 Provider 列表中选择一个不在排除列表中的替代
    pub fn select_alternative_excluding(
        &self,
        excluded_providers: &HashSet<ProviderType>,
        available_providers: &[ProviderType],
    ) -> Option<ProviderType> {
        available_providers
            .iter()
            .find(|p| !excluded_providers.contains(p))
            .copied()
    }

    /// 检查是否为配额超限错误
    pub fn is_quota_exceeded(status_code: Option<u16>, error_message: &str) -> bool {
        FailureType::detect(status_code, error_message).is_quota_exceeded()
    }
}

impl Default for Failover {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// 故障转移管理器
///
/// 管理 Provider 故障转移状态，跟踪失败的 Provider 并协调切换
#[derive(Debug)]
pub struct FailoverManager {
    failover: Failover,
    /// 当前请求中已失败的 Provider 集合
    failed_providers: HashSet<ProviderType>,
    /// 切换日志
    switch_log: Vec<SwitchEvent>,
}

/// 切换事件
#[derive(Debug, Clone)]
pub struct SwitchEvent {
    /// 失败的 Provider
    pub from_provider: ProviderType,
    /// 切换到的 Provider
    pub to_provider: ProviderType,
    /// 故障类型
    pub failure_type: FailureType,
    /// 时间戳
    pub timestamp: std::time::Instant,
}

impl FailoverManager {
    /// 创建新的故障转移管理器
    pub fn new(config: FailoverConfig) -> Self {
        Self {
            failover: Failover::new(config),
            failed_providers: HashSet::new(),
            switch_log: Vec::new(),
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(FailoverConfig::default())
    }

    /// 获取配置
    pub fn config(&self) -> &FailoverConfig {
        self.failover.config()
    }

    /// 重置状态（新请求开始时调用）
    pub fn reset(&mut self) {
        self.failed_providers.clear();
    }

    /// 处理 Provider 失败并尝试切换
    ///
    /// # Arguments
    /// * `failed_provider` - 失败的 Provider
    /// * `status_code` - HTTP 状态码
    /// * `error_message` - 错误消息
    /// * `available_providers` - 所有可用的 Provider 列表
    ///
    /// # Returns
    /// 故障转移结果
    pub fn handle_failure_and_switch(
        &mut self,
        failed_provider: ProviderType,
        status_code: Option<u16>,
        error_message: &str,
        available_providers: &[ProviderType],
    ) -> FailoverResult {
        // 记录失败的 Provider
        self.failed_providers.insert(failed_provider);

        // 检测故障类型
        let failure_type = FailureType::detect(status_code, error_message);

        // 检查是否启用自动切换
        if !self.failover.config().auto_switch {
            return FailoverResult::not_switched(failure_type, "自动切换已禁用");
        }

        // 检查是否应该在此类故障时切换
        let should_switch = match &failure_type {
            FailureType::QuotaExceeded => self.failover.config().switch_on_quota,
            FailureType::ServiceUnavailable => true,
            FailureType::AuthenticationFailed => false,
            FailureType::Other => false,
        };

        if !should_switch {
            return FailoverResult::not_switched(
                failure_type,
                &format!("不在 {:?} 故障时切换", failure_type),
            );
        }

        // 选择替代 Provider（排除所有已失败的）
        match self
            .failover
            .select_alternative_excluding(&self.failed_providers, available_providers)
        {
            Some(new_provider) => {
                // 记录切换事件
                self.switch_log.push(SwitchEvent {
                    from_provider: failed_provider,
                    to_provider: new_provider,
                    failure_type,
                    timestamp: std::time::Instant::now(),
                });

                FailoverResult::switched(new_provider, failure_type)
            }
            None => FailoverResult::not_switched(failure_type, "没有可用的替代 Provider"),
        }
    }

    /// 获取已失败的 Provider 列表
    pub fn failed_providers(&self) -> &HashSet<ProviderType> {
        &self.failed_providers
    }

    /// 获取切换日志
    pub fn switch_log(&self) -> &[SwitchEvent] {
        &self.switch_log
    }

    /// 清除切换日志
    pub fn clear_switch_log(&mut self) {
        self.switch_log.clear();
    }

    /// 检查 Provider 是否已失败
    pub fn is_provider_failed(&self, provider: ProviderType) -> bool {
        self.failed_providers.contains(&provider)
    }

    /// 获取切换次数
    pub fn switch_count(&self) -> usize {
        self.switch_log.len()
    }
}

impl Default for FailoverManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_failover_config_default() {
        let config = FailoverConfig::default();
        assert!(config.auto_switch);
        assert!(config.switch_on_quota);
    }

    #[test]
    fn test_failover_config_disabled() {
        let config = FailoverConfig::disabled();
        assert!(!config.auto_switch);
        assert!(!config.switch_on_quota);
    }

    #[test]
    fn test_failure_type_detect_quota_by_status() {
        let failure_type = FailureType::detect(Some(429), "");
        assert_eq!(failure_type, FailureType::QuotaExceeded);
    }

    #[test]
    fn test_failure_type_detect_quota_by_message() {
        let test_cases = vec![
            "Rate limit exceeded",
            "Quota exceeded for this API",
            "Too many requests",
            "Request was throttled",
        ];

        for msg in test_cases {
            let failure_type = FailureType::detect(Some(400), msg);
            assert_eq!(
                failure_type,
                FailureType::QuotaExceeded,
                "Failed for message: {}",
                msg
            );
        }
    }

    #[test]
    fn test_failure_type_detect_auth_failed() {
        assert_eq!(
            FailureType::detect(Some(401), "Unauthorized"),
            FailureType::AuthenticationFailed
        );
        assert_eq!(
            FailureType::detect(Some(403), "Forbidden"),
            FailureType::AuthenticationFailed
        );
    }

    #[test]
    fn test_failure_type_detect_service_unavailable() {
        assert_eq!(
            FailureType::detect(Some(502), "Bad Gateway"),
            FailureType::ServiceUnavailable
        );
        assert_eq!(
            FailureType::detect(Some(503), "Service Unavailable"),
            FailureType::ServiceUnavailable
        );
        assert_eq!(
            FailureType::detect(Some(504), "Gateway Timeout"),
            FailureType::ServiceUnavailable
        );
    }

    #[test]
    fn test_failure_type_detect_other() {
        assert_eq!(
            FailureType::detect(Some(400), "Bad Request"),
            FailureType::Other
        );
        assert_eq!(
            FailureType::detect(Some(404), "Not Found"),
            FailureType::Other
        );
    }

    #[test]
    fn test_handle_failure_quota_exceeded() {
        let failover = Failover::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini, ProviderType::Qwen];

        let result = failover.handle_failure(
            ProviderType::Kiro,
            Some(429),
            "Rate limit exceeded",
            &available,
        );

        assert!(result.switched);
        assert_eq!(result.new_provider, Some(ProviderType::Gemini));
        assert_eq!(result.failure_type, FailureType::QuotaExceeded);
    }

    #[test]
    fn test_handle_failure_auto_switch_disabled() {
        let failover = Failover::new(FailoverConfig::disabled());
        let available = vec![ProviderType::Kiro, ProviderType::Gemini];

        let result = failover.handle_failure(
            ProviderType::Kiro,
            Some(429),
            "Rate limit exceeded",
            &available,
        );

        assert!(!result.switched);
        assert!(result.new_provider.is_none());
    }

    #[test]
    fn test_handle_failure_no_alternative() {
        let failover = Failover::with_defaults();
        let available = vec![ProviderType::Kiro]; // 只有一个 Provider

        let result = failover.handle_failure(
            ProviderType::Kiro,
            Some(429),
            "Rate limit exceeded",
            &available,
        );

        assert!(!result.switched);
        assert!(result.new_provider.is_none());
    }

    #[test]
    fn test_handle_failure_auth_failed_no_switch() {
        let failover = Failover::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini];

        let result =
            failover.handle_failure(ProviderType::Kiro, Some(401), "Unauthorized", &available);

        assert!(!result.switched);
        assert!(result.new_provider.is_none());
        assert_eq!(result.failure_type, FailureType::AuthenticationFailed);
    }

    #[test]
    fn test_handle_failure_service_unavailable() {
        let failover = Failover::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini];

        let result = failover.handle_failure(
            ProviderType::Kiro,
            Some(503),
            "Service Unavailable",
            &available,
        );

        assert!(result.switched);
        assert_eq!(result.new_provider, Some(ProviderType::Gemini));
        assert_eq!(result.failure_type, FailureType::ServiceUnavailable);
    }

    #[test]
    fn test_select_alternative() {
        let failover = Failover::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini, ProviderType::Qwen];

        // 排除 Kiro，应该选择 Gemini
        let result = failover.select_alternative(ProviderType::Kiro, &available);
        assert_eq!(result, Some(ProviderType::Gemini));

        // 排除 Gemini，应该选择 Kiro
        let result = failover.select_alternative(ProviderType::Gemini, &available);
        assert_eq!(result, Some(ProviderType::Kiro));
    }

    #[test]
    fn test_select_alternative_excluding() {
        let failover = Failover::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini, ProviderType::Qwen];

        let mut excluded = HashSet::new();
        excluded.insert(ProviderType::Kiro);
        excluded.insert(ProviderType::Gemini);

        let result = failover.select_alternative_excluding(&excluded, &available);
        assert_eq!(result, Some(ProviderType::Qwen));
    }

    #[test]
    fn test_is_quota_exceeded() {
        assert!(Failover::is_quota_exceeded(Some(429), ""));
        assert!(Failover::is_quota_exceeded(
            Some(400),
            "Rate limit exceeded"
        ));
        assert!(!Failover::is_quota_exceeded(Some(400), "Bad Request"));
        assert!(!Failover::is_quota_exceeded(
            Some(500),
            "Internal Server Error"
        ));
    }
}

#[cfg(test)]
mod manager_tests {
    use super::*;

    #[test]
    fn test_failover_manager_new() {
        let manager = FailoverManager::with_defaults();
        assert!(manager.config().auto_switch);
        assert!(manager.failed_providers().is_empty());
        assert!(manager.switch_log().is_empty());
    }

    #[test]
    fn test_failover_manager_reset() {
        let mut manager = FailoverManager::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini];

        // 触发一次失败
        manager.handle_failure_and_switch(ProviderType::Kiro, Some(429), "Rate limit", &available);

        assert!(!manager.failed_providers().is_empty());

        // 重置
        manager.reset();
        assert!(manager.failed_providers().is_empty());
    }

    #[test]
    fn test_failover_manager_tracks_failed_providers() {
        let mut manager = FailoverManager::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini, ProviderType::Qwen];

        // 第一次失败
        let result = manager.handle_failure_and_switch(
            ProviderType::Kiro,
            Some(429),
            "Rate limit",
            &available,
        );
        assert!(result.switched);
        assert_eq!(result.new_provider, Some(ProviderType::Gemini));
        assert!(manager.is_provider_failed(ProviderType::Kiro));

        // 第二次失败（Gemini 也失败了）
        let result = manager.handle_failure_and_switch(
            ProviderType::Gemini,
            Some(429),
            "Rate limit",
            &available,
        );
        assert!(result.switched);
        assert_eq!(result.new_provider, Some(ProviderType::Qwen));
        assert!(manager.is_provider_failed(ProviderType::Gemini));

        // 第三次失败（所有 Provider 都失败了）
        let result = manager.handle_failure_and_switch(
            ProviderType::Qwen,
            Some(429),
            "Rate limit",
            &available,
        );
        assert!(!result.switched);
        assert!(result.new_provider.is_none());
    }

    #[test]
    fn test_failover_manager_switch_log() {
        let mut manager = FailoverManager::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini, ProviderType::Qwen];

        // 触发两次切换
        manager.handle_failure_and_switch(ProviderType::Kiro, Some(429), "Rate limit", &available);
        manager.handle_failure_and_switch(
            ProviderType::Gemini,
            Some(503),
            "Service Unavailable",
            &available,
        );

        assert_eq!(manager.switch_count(), 2);

        let log = manager.switch_log();
        assert_eq!(log[0].from_provider, ProviderType::Kiro);
        assert_eq!(log[0].to_provider, ProviderType::Gemini);
        assert_eq!(log[0].failure_type, FailureType::QuotaExceeded);

        assert_eq!(log[1].from_provider, ProviderType::Gemini);
        assert_eq!(log[1].to_provider, ProviderType::Qwen);
        assert_eq!(log[1].failure_type, FailureType::ServiceUnavailable);
    }

    #[test]
    fn test_failover_manager_clear_switch_log() {
        let mut manager = FailoverManager::with_defaults();
        let available = vec![ProviderType::Kiro, ProviderType::Gemini];

        manager.handle_failure_and_switch(ProviderType::Kiro, Some(429), "Rate limit", &available);

        assert_eq!(manager.switch_count(), 1);

        manager.clear_switch_log();
        assert_eq!(manager.switch_count(), 0);
    }

    #[test]
    fn test_failover_manager_disabled() {
        let mut manager = FailoverManager::new(FailoverConfig::disabled());
        let available = vec![ProviderType::Kiro, ProviderType::Gemini];

        let result = manager.handle_failure_and_switch(
            ProviderType::Kiro,
            Some(429),
            "Rate limit",
            &available,
        );

        assert!(!result.switched);
        assert!(result.new_provider.is_none());
        // 即使禁用，也应该记录失败的 Provider
        assert!(manager.is_provider_failed(ProviderType::Kiro));
    }

    #[test]
    fn test_failover_manager_quota_switch_disabled() {
        let config = FailoverConfig {
            auto_switch: true,
            switch_on_quota: false,
        };
        let mut manager = FailoverManager::new(config);
        let available = vec![ProviderType::Kiro, ProviderType::Gemini];

        // 配额超限不应切换
        let result = manager.handle_failure_and_switch(
            ProviderType::Kiro,
            Some(429),
            "Rate limit",
            &available,
        );
        assert!(!result.switched);

        // 但服务不可用应该切换
        manager.reset();
        let result = manager.handle_failure_and_switch(
            ProviderType::Kiro,
            Some(503),
            "Service Unavailable",
            &available,
        );
        assert!(result.switched);
    }
}
