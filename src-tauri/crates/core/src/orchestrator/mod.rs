//! 模型编排器模块
//!
//! 提供 Mini/Pro/Max 服务等级的智能路由系统。
//!
//! ## 模块结构
//!
//! - `tier` - 服务等级定义 (Mini/Pro/Max)
//! - `strategy` - 选择策略 trait 和注册表
//! - `strategies` - 内置策略实现
//! - `selector` - 模型选择器
//! - `fallback` - 降级处理器
//! - `pool_builder` - 动态模型池构建
//! - `orchestrator` - 统一编排接口
//!
//! ## 使用模式
//!
//! 1. **简单模式（默认）**: Mini/Pro/Max 三档，动态根据用户凭证组合模型池
//! 2. **专家模式**: 直接选择具体模型

mod fallback;
mod model_orchestrator;
mod pool_builder;
mod selector;
pub mod strategies;
mod strategy;
mod tier;

pub use fallback::{FallbackHandler, FallbackPolicy, FallbackResult};
pub use model_orchestrator::{
    get_global_orchestrator, init_global_orchestrator, ModelOrchestrator, OrchestratorConfig,
    PoolStats,
};
pub use pool_builder::{
    builtin_model_metadata, builtin_provider_definitions, CredentialInfo, DynamicPoolBuilder,
    ModelFamily, ModelMetadata, ProviderDefinition, ProviderType,
};
pub use selector::{ModelSelector, SelectionResult};
pub use strategies::*;
pub use strategy::{
    ModelSelection, SelectionContext, SelectionStrategy, StrategyError, StrategyInfo,
    StrategyRegistry, StrategyResult, TaskHint,
};
pub use tier::{AvailableModel, ServiceTier, TierConfig, TierPool};
