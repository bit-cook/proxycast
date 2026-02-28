//! 模型编排器
//!
//! 统一的模型编排接口，整合模型池构建、策略选择和降级处理。

use super::fallback::{FallbackHandler, FallbackPolicy};
use super::pool_builder::{CredentialInfo, DynamicPoolBuilder};
use super::selector::{ModelSelector, SelectionResult};
use super::strategies::create_default_registry;
use super::strategy::{SelectionContext, StrategyInfo, StrategyResult, TaskHint};
use super::tier::{AvailableModel, ServiceTier, TierPool};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 编排器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// 默认服务等级
    pub default_tier: ServiceTier,
    /// 是否启用自动降级
    pub auto_fallback: bool,
    /// 降级策略
    pub fallback_policy: FallbackPolicy,
    /// 是否启用负载均衡
    pub load_balancing: bool,
    /// 模型池刷新间隔（秒）
    pub pool_refresh_interval: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            default_tier: ServiceTier::Pro,
            auto_fallback: true,
            fallback_policy: FallbackPolicy::NextTier,
            load_balancing: true,
            pool_refresh_interval: 60,
        }
    }
}

/// 模型编排器
///
/// 提供统一的模型选择和管理接口
pub struct ModelOrchestrator {
    /// 配置
    config: RwLock<OrchestratorConfig>,
    /// 模型选择器
    selector: ModelSelector,
    /// 模型池构建器
    pool_builder: DynamicPoolBuilder,
    /// 降级处理器（预留，将来实现自动降级功能）
    #[allow(dead_code)]
    fallback_handler: FallbackHandler,
    /// 当前凭证列表
    credentials: RwLock<Vec<CredentialInfo>>,
}

impl ModelOrchestrator {
    /// 创建新的编排器
    pub fn new() -> Self {
        let registry = create_default_registry();
        let config = OrchestratorConfig::default();

        Self {
            fallback_handler: FallbackHandler::new(config.fallback_policy),
            config: RwLock::new(config),
            selector: ModelSelector::new(registry),
            pool_builder: DynamicPoolBuilder::new(),
            credentials: RwLock::new(Vec::new()),
        }
    }

    /// 使用自定义配置创建
    pub fn with_config(config: OrchestratorConfig) -> Self {
        let registry = create_default_registry();

        Self {
            fallback_handler: FallbackHandler::new(config.fallback_policy),
            config: RwLock::new(config),
            selector: ModelSelector::new(registry),
            pool_builder: DynamicPoolBuilder::new(),
            credentials: RwLock::new(Vec::new()),
        }
    }

    /// 更新配置
    pub async fn update_config(&self, config: OrchestratorConfig) {
        let mut current = self.config.write().await;
        *current = config;
        info!("编排器配置已更新");
    }

    /// 获取配置
    pub async fn get_config(&self) -> OrchestratorConfig {
        self.config.read().await.clone()
    }

    /// 更新凭证列表
    pub async fn update_credentials(&self, credentials: Vec<CredentialInfo>) {
        info!("更新凭证列表: {} 个凭证", credentials.len());

        // 构建新的模型池
        let pool = self.pool_builder.build_pool(&credentials);

        info!(
            "模型池已构建: Mini={}, Pro={}, Max={}",
            pool.get(ServiceTier::Mini).len(),
            pool.get(ServiceTier::Pro).len(),
            pool.get(ServiceTier::Max).len()
        );

        // 更新选择器的模型池
        self.selector.update_pool(pool).await;

        // 保存凭证列表
        let mut creds = self.credentials.write().await;
        *creds = credentials;
    }

    /// 添加凭证
    pub async fn add_credential(&self, credential: CredentialInfo) {
        let mut creds = self.credentials.write().await;
        creds.push(credential);

        // 重新构建模型池
        let pool = self.pool_builder.build_pool(&creds);
        drop(creds);

        self.selector.update_pool(pool).await;
    }

    /// 移除凭证
    pub async fn remove_credential(&self, credential_id: &str) {
        let mut creds = self.credentials.write().await;
        creds.retain(|c| c.id != credential_id);

        // 重新构建模型池
        let pool = self.pool_builder.build_pool(&creds);
        drop(creds);

        self.selector.update_pool(pool).await;
    }

    /// 选择模型
    pub async fn select(&self, ctx: &SelectionContext) -> StrategyResult<SelectionResult> {
        debug!("选择模型: 等级={}, 任务={:?}", ctx.tier, ctx.task_hint);

        self.selector.select(ctx).await
    }

    /// 使用指定策略选择模型
    pub async fn select_with_strategy(
        &self,
        strategy_id: &str,
        ctx: &SelectionContext,
    ) -> StrategyResult<SelectionResult> {
        self.selector.select_with_strategy(strategy_id, ctx).await
    }

    /// 快速选择（使用默认等级和策略）
    pub async fn quick_select(&self) -> StrategyResult<SelectionResult> {
        let config = self.config.read().await;
        let ctx = SelectionContext::new(config.default_tier);
        drop(config);

        self.select(&ctx).await
    }

    /// 为特定任务选择模型
    pub async fn select_for_task(
        &self,
        tier: ServiceTier,
        task: TaskHint,
    ) -> StrategyResult<SelectionResult> {
        let ctx = SelectionContext::new(tier).with_task_hint(task);
        self.select(&ctx).await
    }

    /// 获取当前模型池
    pub async fn get_pool(&self) -> TierPool {
        self.selector.get_pool().await
    }

    /// 获取指定等级的可用模型
    pub async fn get_models(&self, tier: ServiceTier) -> Vec<AvailableModel> {
        let pool = self.selector.get_pool().await;
        pool.get(tier).to_vec()
    }

    /// 获取所有可用模型
    pub async fn get_all_models(&self) -> Vec<AvailableModel> {
        let pool = self.selector.get_pool().await;
        let mut all = Vec::new();
        all.extend(pool.get(ServiceTier::Mini).iter().cloned());
        all.extend(pool.get(ServiceTier::Pro).iter().cloned());
        all.extend(pool.get(ServiceTier::Max).iter().cloned());
        all
    }

    /// 列出所有可用策略
    pub async fn list_strategies(&self) -> Vec<StrategyInfo> {
        self.selector.list_strategies().await
    }

    /// 设置等级的默认策略
    pub fn set_tier_strategy(&mut self, tier: ServiceTier, strategy_id: &str) {
        self.selector.set_tier_strategy(tier, strategy_id);
    }

    /// 获取模型池统计
    pub async fn get_pool_stats(&self) -> PoolStats {
        let pool = self.selector.get_pool().await;

        PoolStats {
            mini_count: pool.get(ServiceTier::Mini).len(),
            pro_count: pool.get(ServiceTier::Pro).len(),
            max_count: pool.get(ServiceTier::Max).len(),
            total_count: pool.total_count(),
            healthy_count: pool
                .get(ServiceTier::Mini)
                .iter()
                .chain(pool.get(ServiceTier::Pro).iter())
                .chain(pool.get(ServiceTier::Max).iter())
                .filter(|m| m.is_healthy)
                .count(),
        }
    }

    /// 标记模型为不健康
    pub async fn mark_unhealthy(&self, model_id: &str, credential_id: &str) {
        warn!("标记模型为不健康: {} (凭证: {})", model_id, credential_id);

        let mut creds = self.credentials.write().await;
        if let Some(cred) = creds.iter_mut().find(|c| c.id == credential_id) {
            cred.is_healthy = false;
        }

        // 重新构建模型池
        let pool = self.pool_builder.build_pool(&creds);
        drop(creds);

        self.selector.update_pool(pool).await;
    }

    /// 标记模型为健康
    pub async fn mark_healthy(&self, credential_id: &str) {
        info!("标记凭证为健康: {}", credential_id);

        let mut creds = self.credentials.write().await;
        if let Some(cred) = creds.iter_mut().find(|c| c.id == credential_id) {
            cred.is_healthy = true;
        }

        // 重新构建模型池
        let pool = self.pool_builder.build_pool(&creds);
        drop(creds);

        self.selector.update_pool(pool).await;
    }

    /// 更新凭证负载
    pub async fn update_load(&self, credential_id: &str, load: u8) {
        let mut creds = self.credentials.write().await;
        if let Some(cred) = creds.iter_mut().find(|c| c.id == credential_id) {
            cred.current_load = Some(load);
        }

        // 重新构建模型池
        let pool = self.pool_builder.build_pool(&creds);
        drop(creds);

        self.selector.update_pool(pool).await;
    }
}

impl Default for ModelOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// 模型池统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// Mini 等级模型数
    pub mini_count: usize,
    /// Pro 等级模型数
    pub pro_count: usize,
    /// Max 等级模型数
    pub max_count: usize,
    /// 总模型数
    pub total_count: usize,
    /// 健康模型数
    pub healthy_count: usize,
}

/// 全局编排器实例
static GLOBAL_ORCHESTRATOR: std::sync::OnceLock<Arc<ModelOrchestrator>> =
    std::sync::OnceLock::new();

/// 初始化全局编排器
pub fn init_global_orchestrator() -> Arc<ModelOrchestrator> {
    GLOBAL_ORCHESTRATOR
        .get_or_init(|| Arc::new(ModelOrchestrator::new()))
        .clone()
}

/// 获取全局编排器
pub fn get_global_orchestrator() -> Option<Arc<ModelOrchestrator>> {
    GLOBAL_ORCHESTRATOR.get().cloned()
}

#[cfg(test)]
mod tests {
    use super::super::pool_builder::ProviderType;
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_basic() {
        let orchestrator = ModelOrchestrator::new();

        // 添加凭证
        orchestrator
            .update_credentials(vec![CredentialInfo {
                id: "cred-1".to_string(),
                provider_type: ProviderType::Anthropic,
                original_provider_type: None,
                supported_models: vec![
                    "claude-sonnet-4-5-20250514".to_string(),
                    "claude-3-5-haiku-20241022".to_string(),
                ],
                is_healthy: true,
                current_load: Some(30),
            }])
            .await;

        // 获取统计
        let stats = orchestrator.get_pool_stats().await;
        assert!(stats.total_count > 0);

        // 选择模型
        let ctx = SelectionContext::new(ServiceTier::Pro);
        let result = orchestrator.select(&ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_orchestrator_task_selection() {
        let orchestrator = ModelOrchestrator::new();

        orchestrator
            .update_credentials(vec![CredentialInfo {
                id: "cred-1".to_string(),
                provider_type: ProviderType::Anthropic,
                original_provider_type: None,
                supported_models: vec![
                    "claude-sonnet-4-5-20250514".to_string(),
                    "claude-3-5-haiku-20241022".to_string(),
                ],
                is_healthy: true,
                current_load: Some(30),
            }])
            .await;

        // 为代码任务选择
        let result = orchestrator
            .select_for_task(ServiceTier::Pro, TaskHint::Coding)
            .await;
        assert!(result.is_ok());
    }
}
