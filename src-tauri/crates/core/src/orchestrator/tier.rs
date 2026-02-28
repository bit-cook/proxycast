//! 服务等级定义
//!
//! 定义 Mini/Pro/Max 三个服务等级及其配置。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 服务等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ServiceTier {
    /// Mini - 快速响应，适合简单任务
    Mini,
    /// Pro - 均衡选择，适合大多数任务
    #[default]
    Pro,
    /// Max - 最强能力，适合复杂任务
    Max,
}

impl ServiceTier {
    /// 获取等级的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ServiceTier::Mini => "Mini",
            ServiceTier::Pro => "Pro",
            ServiceTier::Max => "Max",
        }
    }

    /// 获取等级的描述
    pub fn description(&self) -> &'static str {
        match self {
            ServiceTier::Mini => "快速响应，适合简单任务",
            ServiceTier::Pro => "均衡选择，适合大多数任务",
            ServiceTier::Max => "最强能力，适合复杂任务",
        }
    }

    /// 获取等级的数值（用于排序）
    pub fn level(&self) -> u8 {
        match self {
            ServiceTier::Mini => 1,
            ServiceTier::Pro => 2,
            ServiceTier::Max => 3,
        }
    }

    /// 从字符串解析
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mini" => Some(ServiceTier::Mini),
            "pro" => Some(ServiceTier::Pro),
            "max" => Some(ServiceTier::Max),
            _ => None,
        }
    }

    /// 获取所有等级
    pub fn all() -> &'static [ServiceTier] {
        &[ServiceTier::Mini, ServiceTier::Pro, ServiceTier::Max]
    }
}

impl std::fmt::Display for ServiceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// 等级配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    /// 等级
    pub tier: ServiceTier,
    /// 默认策略 ID
    pub default_strategy: String,
    /// 模型家族优先级（按优先级排序）
    pub family_priorities: Vec<String>,
    /// 最大并发请求数
    pub max_concurrent: Option<u32>,
    /// 超时时间（毫秒）
    pub timeout_ms: Option<u64>,
}

impl TierConfig {
    /// 创建 Mini 等级的默认配置
    pub fn mini() -> Self {
        Self {
            tier: ServiceTier::Mini,
            default_strategy: "speed_optimized".to_string(),
            family_priorities: vec![
                "haiku".to_string(),
                "flash".to_string(),
                "gpt-3.5".to_string(),
            ],
            max_concurrent: Some(10),
            timeout_ms: Some(30000),
        }
    }

    /// 创建 Pro 等级的默认配置
    pub fn pro() -> Self {
        Self {
            tier: ServiceTier::Pro,
            default_strategy: "load_balanced".to_string(),
            family_priorities: vec!["sonnet".to_string(), "pro".to_string(), "gpt-4".to_string()],
            max_concurrent: Some(5),
            timeout_ms: Some(120000),
        }
    }

    /// 创建 Max 等级的默认配置
    pub fn max() -> Self {
        Self {
            tier: ServiceTier::Max,
            default_strategy: "task_based".to_string(),
            family_priorities: vec!["opus".to_string(), "ultra".to_string(), "o1".to_string()],
            max_concurrent: Some(3),
            timeout_ms: Some(300000),
        }
    }

    /// 获取默认配置映射
    pub fn defaults() -> HashMap<ServiceTier, TierConfig> {
        let mut map = HashMap::new();
        map.insert(ServiceTier::Mini, TierConfig::mini());
        map.insert(ServiceTier::Pro, TierConfig::pro());
        map.insert(ServiceTier::Max, TierConfig::max());
        map
    }
}

/// 可用模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableModel {
    /// 模型 ID
    pub id: String,
    /// 显示名称
    pub display_name: String,
    /// Provider 类型
    pub provider_type: String,
    /// 模型家族
    pub family: Option<String>,
    /// 凭证 ID
    pub credential_id: String,
    /// 上下文长度
    pub context_length: Option<u32>,
    /// 是否支持视觉
    pub supports_vision: bool,
    /// 是否支持工具调用
    pub supports_tools: bool,
    /// 输入价格（每 1M tokens）
    pub input_cost_per_million: Option<f64>,
    /// 输出价格（每 1M tokens）
    pub output_cost_per_million: Option<f64>,
    /// 健康状态
    pub is_healthy: bool,
    /// 当前负载（0-100）
    pub current_load: Option<u8>,
}

impl AvailableModel {
    /// 计算模型的综合评分
    pub fn score(&self, tier: ServiceTier) -> f64 {
        let mut score = 0.0;

        // 基础分：健康状态
        if !self.is_healthy {
            return 0.0;
        }

        // 家族匹配分
        if let Some(family) = &self.family {
            let tier_families = match tier {
                ServiceTier::Mini => vec!["haiku", "flash", "gpt-3.5"],
                ServiceTier::Pro => vec!["sonnet", "pro", "gpt-4"],
                ServiceTier::Max => vec!["opus", "ultra", "o1"],
            };

            for (i, f) in tier_families.iter().enumerate() {
                if family.to_lowercase().contains(f) {
                    score += 100.0 - (i as f64 * 10.0);
                    break;
                }
            }
        }

        // 负载分（负载越低越好）
        if let Some(load) = self.current_load {
            score += (100 - load) as f64 * 0.5;
        } else {
            score += 50.0; // 默认中等负载
        }

        // 能力分
        if self.supports_vision {
            score += 10.0;
        }
        if self.supports_tools {
            score += 10.0;
        }

        // 上下文长度分
        if let Some(ctx) = self.context_length {
            score += (ctx as f64 / 10000.0).min(20.0);
        }

        score
    }
}

/// 等级模型池
#[derive(Debug, Clone, Default)]
pub struct TierPool {
    /// Mini 等级可用模型
    pub mini: Vec<AvailableModel>,
    /// Pro 等级可用模型
    pub pro: Vec<AvailableModel>,
    /// Max 等级可用模型
    pub max: Vec<AvailableModel>,
}

impl TierPool {
    /// 创建新的模型池
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取指定等级的模型列表
    pub fn get(&self, tier: ServiceTier) -> &[AvailableModel] {
        match tier {
            ServiceTier::Mini => &self.mini,
            ServiceTier::Pro => &self.pro,
            ServiceTier::Max => &self.max,
        }
    }

    /// 获取指定等级的可变模型列表
    pub fn get_mut(&mut self, tier: ServiceTier) -> &mut Vec<AvailableModel> {
        match tier {
            ServiceTier::Mini => &mut self.mini,
            ServiceTier::Pro => &mut self.pro,
            ServiceTier::Max => &mut self.max,
        }
    }

    /// 添加模型到指定等级
    pub fn add(&mut self, tier: ServiceTier, model: AvailableModel) {
        self.get_mut(tier).push(model);
    }

    /// 获取所有等级的模型总数
    pub fn total_count(&self) -> usize {
        self.mini.len() + self.pro.len() + self.max.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.mini.is_empty() && self.pro.is_empty() && self.max.is_empty()
    }

    /// 按评分排序所有等级的模型
    pub fn sort_by_score(&mut self) {
        self.mini.sort_by(|a, b| {
            b.score(ServiceTier::Mini)
                .partial_cmp(&a.score(ServiceTier::Mini))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.pro.sort_by(|a, b| {
            b.score(ServiceTier::Pro)
                .partial_cmp(&a.score(ServiceTier::Pro))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.max.sort_by(|a, b| {
            b.score(ServiceTier::Max)
                .partial_cmp(&a.score(ServiceTier::Max))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_tier_basics() {
        assert_eq!(ServiceTier::Mini.level(), 1);
        assert_eq!(ServiceTier::Pro.level(), 2);
        assert_eq!(ServiceTier::Max.level(), 3);

        assert_eq!(ServiceTier::parse_str("mini"), Some(ServiceTier::Mini));
        assert_eq!(ServiceTier::parse_str("PRO"), Some(ServiceTier::Pro));
        assert_eq!(ServiceTier::parse_str("invalid"), None);
    }

    #[test]
    fn test_tier_config_defaults() {
        let defaults = TierConfig::defaults();
        assert_eq!(defaults.len(), 3);
        assert!(defaults.contains_key(&ServiceTier::Mini));
        assert!(defaults.contains_key(&ServiceTier::Pro));
        assert!(defaults.contains_key(&ServiceTier::Max));
    }

    #[test]
    fn test_available_model_score() {
        let model = AvailableModel {
            id: "claude-3-5-haiku".to_string(),
            display_name: "Claude 3.5 Haiku".to_string(),
            provider_type: "anthropic".to_string(),
            family: Some("haiku".to_string()),
            credential_id: "cred-1".to_string(),
            context_length: Some(200000),
            supports_vision: true,
            supports_tools: true,
            input_cost_per_million: None,
            output_cost_per_million: None,
            is_healthy: true,
            current_load: Some(30),
        };

        // Haiku 模型在 Mini 等级应该得分最高
        let mini_score = model.score(ServiceTier::Mini);
        let pro_score = model.score(ServiceTier::Pro);

        assert!(mini_score > pro_score);
    }

    #[test]
    fn test_tier_pool() {
        let mut pool = TierPool::new();
        assert!(pool.is_empty());

        pool.add(
            ServiceTier::Mini,
            AvailableModel {
                id: "test".to_string(),
                display_name: "Test".to_string(),
                provider_type: "test".to_string(),
                family: None,
                credential_id: "cred".to_string(),
                context_length: None,
                supports_vision: false,
                supports_tools: false,
                input_cost_per_million: None,
                output_cost_per_million: None,
                is_healthy: true,
                current_load: None,
            },
        );

        assert!(!pool.is_empty());
        assert_eq!(pool.total_count(), 1);
        assert_eq!(pool.get(ServiceTier::Mini).len(), 1);
    }
}
