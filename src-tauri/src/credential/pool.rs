//! 凭证池实现
//!
//! 使用 DashMap 实现线程安全的凭证池管理

use super::types::{Credential, CredentialStatus};
use crate::ProviderType;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

/// 凭证池 - 管理同一 Provider 的多个凭证
pub struct CredentialPool {
    /// 所属 Provider 类型
    provider: ProviderType,
    /// 凭证存储（id -> Credential）
    pub(crate) credentials: DashMap<String, Credential>,
    /// 轮询索引（用于负载均衡）
    round_robin_index: AtomicUsize,
}

/// 凭证池状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    /// Provider 类型
    pub provider: ProviderType,
    /// 总凭证数
    pub total: usize,
    /// 活跃凭证数
    pub active: usize,
    /// 冷却中凭证数
    pub cooldown: usize,
    /// 不健康凭证数
    pub unhealthy: usize,
    /// 已禁用凭证数
    pub disabled: usize,
}

/// 凭证池错误
#[derive(Debug, Clone, PartialEq)]
pub enum PoolError {
    /// 凭证已存在
    CredentialExists(String),
    /// 凭证不存在
    CredentialNotFound(String),
    /// 凭证池为空
    EmptyPool,
    /// 所有凭证不可用
    NoAvailableCredential,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::CredentialExists(id) => write!(f, "凭证已存在: {}", id),
            PoolError::CredentialNotFound(id) => write!(f, "凭证不存在: {}", id),
            PoolError::EmptyPool => write!(f, "凭证池为空"),
            PoolError::NoAvailableCredential => write!(f, "没有可用的凭证"),
        }
    }
}

impl std::error::Error for PoolError {}

impl CredentialPool {
    /// 创建新的凭证池
    pub fn new(provider: ProviderType) -> Self {
        Self {
            provider,
            credentials: DashMap::new(),
            round_robin_index: AtomicUsize::new(0),
        }
    }

    /// 获取 Provider 类型
    pub fn provider(&self) -> ProviderType {
        self.provider
    }

    /// 获取凭证池大小
    pub fn len(&self) -> usize {
        self.credentials.len()
    }

    /// 检查凭证池是否为空
    pub fn is_empty(&self) -> bool {
        self.credentials.is_empty()
    }

    /// 添加凭证到池中
    ///
    /// # 错误
    /// - 如果凭证 ID 已存在，返回 `PoolError::CredentialExists`
    pub fn add(&self, credential: Credential) -> Result<(), PoolError> {
        if self.credentials.contains_key(&credential.id) {
            return Err(PoolError::CredentialExists(credential.id.clone()));
        }
        self.credentials.insert(credential.id.clone(), credential);
        Ok(())
    }

    /// 从池中移除凭证
    ///
    /// # 错误
    /// - 如果凭证不存在，返回 `PoolError::CredentialNotFound`
    pub fn remove(&self, id: &str) -> Result<Credential, PoolError> {
        self.credentials
            .remove(id)
            .map(|(_, cred)| cred)
            .ok_or_else(|| PoolError::CredentialNotFound(id.to_string()))
    }

    /// 获取凭证（只读）
    pub fn get(&self, id: &str) -> Option<Credential> {
        self.credentials.get(id).map(|r| r.value().clone())
    }

    /// 获取所有凭证 ID
    pub fn ids(&self) -> Vec<String> {
        self.credentials.iter().map(|r| r.key().clone()).collect()
    }

    /// 获取所有凭证
    pub fn all(&self) -> Vec<Credential> {
        self.credentials.iter().map(|r| r.value().clone()).collect()
    }

    /// 检查凭证是否存在
    pub fn contains(&self, id: &str) -> bool {
        self.credentials.contains_key(id)
    }

    /// 获取池状态
    pub fn status(&self) -> PoolStatus {
        let mut active = 0;
        let mut cooldown = 0;
        let mut unhealthy = 0;
        let mut disabled = 0;

        for entry in self.credentials.iter() {
            match &entry.value().status {
                CredentialStatus::Active => active += 1,
                CredentialStatus::Cooldown { .. } => cooldown += 1,
                CredentialStatus::Unhealthy { .. } => unhealthy += 1,
                CredentialStatus::Disabled => disabled += 1,
            }
        }

        PoolStatus {
            provider: self.provider,
            total: self.credentials.len(),
            active,
            cooldown,
            unhealthy,
            disabled,
        }
    }

    /// 获取活跃凭证数量
    pub fn active_count(&self) -> usize {
        self.credentials
            .iter()
            .filter(|r| r.value().is_available())
            .count()
    }

    /// 标记凭证为冷却状态
    pub fn mark_cooldown(&self, id: &str, duration: Duration) -> Result<(), PoolError> {
        let mut entry = self
            .credentials
            .get_mut(id)
            .ok_or_else(|| PoolError::CredentialNotFound(id.to_string()))?;

        entry.status = CredentialStatus::Cooldown {
            until: Utc::now() + duration,
        };
        Ok(())
    }

    /// 标记凭证为不健康状态
    pub fn mark_unhealthy(&self, id: &str, reason: String) -> Result<(), PoolError> {
        let mut entry = self
            .credentials
            .get_mut(id)
            .ok_or_else(|| PoolError::CredentialNotFound(id.to_string()))?;

        entry.status = CredentialStatus::Unhealthy { reason };
        Ok(())
    }

    /// 恢复凭证为活跃状态
    pub fn mark_active(&self, id: &str) -> Result<(), PoolError> {
        let mut entry = self
            .credentials
            .get_mut(id)
            .ok_or_else(|| PoolError::CredentialNotFound(id.to_string()))?;

        entry.status = CredentialStatus::Active;
        Ok(())
    }

    /// 更新过期的冷却状态
    /// 将冷却期已过的凭证恢复为活跃状态
    pub fn refresh_cooldowns(&self) {
        let now = Utc::now();
        for mut entry in self.credentials.iter_mut() {
            if let CredentialStatus::Cooldown { until } = &entry.status {
                if *until <= now {
                    entry.status = CredentialStatus::Active;
                }
            }
        }
    }

    /// 获取下一个可用凭证（轮询策略）
    ///
    /// # 错误
    /// - 如果池为空，返回 `PoolError::EmptyPool`
    /// - 如果没有可用凭证，返回 `PoolError::NoAvailableCredential`
    pub fn next_available(&self) -> Result<Credential, PoolError> {
        if self.credentials.is_empty() {
            return Err(PoolError::EmptyPool);
        }

        // 先刷新冷却状态
        self.refresh_cooldowns();

        // 收集所有活跃凭证
        let active_creds: Vec<_> = self
            .credentials
            .iter()
            .filter(|r| r.value().is_available())
            .map(|r| r.value().clone())
            .collect();

        if active_creds.is_empty() {
            return Err(PoolError::NoAvailableCredential);
        }

        // 轮询选择
        let index = self.round_robin_index.fetch_add(1, Ordering::SeqCst) % active_creds.len();
        Ok(active_creds[index].clone())
    }

    /// 获取最早恢复时间（当所有凭证都在冷却时）
    pub fn earliest_recovery(&self) -> Option<DateTime<Utc>> {
        self.credentials
            .iter()
            .filter_map(|r| {
                if let CredentialStatus::Cooldown { until } = &r.value().status {
                    Some(*until)
                } else {
                    None
                }
            })
            .min()
    }

    /// 记录凭证使用成功
    pub fn record_success(&self, id: &str, latency_ms: u64) -> Result<(), PoolError> {
        let mut entry = self
            .credentials
            .get_mut(id)
            .ok_or_else(|| PoolError::CredentialNotFound(id.to_string()))?;

        entry.mark_used();
        entry.stats.record_success(latency_ms);
        Ok(())
    }

    /// 记录凭证使用失败
    pub fn record_failure(&self, id: &str) -> Result<(), PoolError> {
        let mut entry = self
            .credentials
            .get_mut(id)
            .ok_or_else(|| PoolError::CredentialNotFound(id.to_string()))?;

        entry.mark_used();
        entry.stats.record_failure();
        Ok(())
    }
}

#[cfg(test)]
mod pool_tests {
    use super::*;
    use crate::credential::CredentialData;

    fn create_test_credential(id: &str) -> Credential {
        Credential::new(
            id.to_string(),
            ProviderType::Kiro,
            CredentialData::ApiKey {
                key: format!("key-{}", id),
                base_url: None,
            },
        )
    }

    #[test]
    fn test_pool_new() {
        let pool = CredentialPool::new(ProviderType::Kiro);
        assert_eq!(pool.provider(), ProviderType::Kiro);
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_pool_add() {
        let pool = CredentialPool::new(ProviderType::Kiro);
        let cred = create_test_credential("test-1");

        assert!(pool.add(cred.clone()).is_ok());
        assert_eq!(pool.len(), 1);
        assert!(pool.contains("test-1"));

        // 重复添加应失败
        let result = pool.add(cred);
        assert!(matches!(result, Err(PoolError::CredentialExists(_))));
    }

    #[test]
    fn test_pool_remove() {
        let pool = CredentialPool::new(ProviderType::Kiro);
        let cred = create_test_credential("test-1");

        pool.add(cred).unwrap();
        assert_eq!(pool.len(), 1);

        let removed = pool.remove("test-1").unwrap();
        assert_eq!(removed.id, "test-1");
        assert!(pool.is_empty());

        // 移除不存在的凭证应失败
        let result = pool.remove("test-1");
        assert!(matches!(result, Err(PoolError::CredentialNotFound(_))));
    }

    #[test]
    fn test_pool_get() {
        let pool = CredentialPool::new(ProviderType::Kiro);
        let cred = create_test_credential("test-1");

        pool.add(cred).unwrap();

        let retrieved = pool.get("test-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-1");

        assert!(pool.get("nonexistent").is_none());
    }

    #[test]
    fn test_pool_status() {
        let pool = CredentialPool::new(ProviderType::Kiro);

        pool.add(create_test_credential("active-1")).unwrap();
        pool.add(create_test_credential("active-2")).unwrap();
        pool.add(create_test_credential("cooldown-1")).unwrap();
        pool.add(create_test_credential("unhealthy-1")).unwrap();

        pool.mark_cooldown("cooldown-1", Duration::hours(1))
            .unwrap();
        pool.mark_unhealthy("unhealthy-1", "test reason".to_string())
            .unwrap();

        let status = pool.status();
        assert_eq!(status.total, 4);
        assert_eq!(status.active, 2);
        assert_eq!(status.cooldown, 1);
        assert_eq!(status.unhealthy, 1);
        assert_eq!(status.disabled, 0);
    }

    #[test]
    fn test_pool_next_available_empty() {
        let pool = CredentialPool::new(ProviderType::Kiro);
        let result = pool.next_available();
        assert!(matches!(result, Err(PoolError::EmptyPool)));
    }

    #[test]
    fn test_pool_next_available_all_cooldown() {
        let pool = CredentialPool::new(ProviderType::Kiro);
        pool.add(create_test_credential("cred-1")).unwrap();
        pool.mark_cooldown("cred-1", Duration::hours(1)).unwrap();

        let result = pool.next_available();
        assert!(matches!(result, Err(PoolError::NoAvailableCredential)));
    }

    #[test]
    fn test_pool_record_success() {
        let pool = CredentialPool::new(ProviderType::Kiro);
        pool.add(create_test_credential("test-1")).unwrap();

        pool.record_success("test-1", 100).unwrap();

        let cred = pool.get("test-1").unwrap();
        assert_eq!(cred.stats.total_requests, 1);
        assert_eq!(cred.stats.successful_requests, 1);
        assert!(cred.last_used.is_some());
    }

    #[test]
    fn test_pool_record_failure() {
        let pool = CredentialPool::new(ProviderType::Kiro);
        pool.add(create_test_credential("test-1")).unwrap();

        pool.record_failure("test-1").unwrap();

        let cred = pool.get("test-1").unwrap();
        assert_eq!(cred.stats.total_requests, 1);
        assert_eq!(cred.stats.consecutive_failures, 1);
    }
}
