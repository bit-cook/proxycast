//! 模型映射器
//!
//! 提供模型别名映射和解析功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    /// 模型 ID
    pub id: String,
    /// 是否为别名
    pub is_alias: bool,
    /// 实际模型名（如果是别名）
    pub actual_model: Option<String>,
}

/// 模型映射器 - 管理模型别名映射
#[derive(Debug, Clone, Default)]
pub struct ModelMapper {
    /// 别名到实际模型的映射 (alias -> actual)
    aliases: HashMap<String, String>,
}

impl ModelMapper {
    /// 创建新的模型映射器
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
        }
    }

    /// 从别名映射创建模型映射器
    pub fn from_aliases(aliases: HashMap<String, String>) -> Self {
        Self { aliases }
    }

    /// 解析模型名（别名 -> 实际名）
    ///
    /// 如果模型名是别名，返回实际模型名；否则返回原模型名
    pub fn resolve(&self, model: &str) -> String {
        self.aliases
            .get(model)
            .cloned()
            .unwrap_or_else(|| model.to_string())
    }

    /// 添加别名映射
    pub fn add_alias(&mut self, alias: &str, actual: &str) {
        self.aliases.insert(alias.to_string(), actual.to_string());
    }

    /// 移除别名映射
    pub fn remove_alias(&mut self, alias: &str) -> Option<String> {
        self.aliases.remove(alias)
    }

    /// 检查是否存在别名
    pub fn has_alias(&self, alias: &str) -> bool {
        self.aliases.contains_key(alias)
    }

    /// 获取别名对应的实际模型（如果存在）
    pub fn get_actual(&self, alias: &str) -> Option<&String> {
        self.aliases.get(alias)
    }

    /// 获取所有别名
    pub fn aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    /// 获取别名数量
    pub fn len(&self) -> usize {
        self.aliases.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }

    /// 获取所有可用模型（包含别名）
    ///
    /// 返回所有别名和实际模型的信息
    pub fn available_models(&self, actual_models: &[String]) -> Vec<ModelInfo> {
        let mut models = Vec::new();

        // 添加实际模型
        for model in actual_models {
            models.push(ModelInfo {
                id: model.clone(),
                is_alias: false,
                actual_model: None,
            });
        }

        // 添加别名
        for (alias, actual) in &self.aliases {
            models.push(ModelInfo {
                id: alias.clone(),
                is_alias: true,
                actual_model: Some(actual.clone()),
            });
        }

        models
    }

    /// 清空所有别名
    pub fn clear(&mut self) {
        self.aliases.clear();
    }
}

#[cfg(test)]
mod mapper_tests {
    use super::*;

    #[test]
    fn test_new_mapper() {
        let mapper = ModelMapper::new();
        assert!(mapper.is_empty());
        assert_eq!(mapper.len(), 0);
    }

    #[test]
    fn test_add_alias() {
        let mut mapper = ModelMapper::new();
        mapper.add_alias("gpt-4", "claude-sonnet-4-5-20250514");

        assert!(mapper.has_alias("gpt-4"));
        assert_eq!(
            mapper.get_actual("gpt-4"),
            Some(&"claude-sonnet-4-5-20250514".to_string())
        );
    }

    #[test]
    fn test_resolve_alias() {
        let mut mapper = ModelMapper::new();
        mapper.add_alias("gpt-4", "claude-sonnet-4-5-20250514");

        // 别名应解析为实际模型
        assert_eq!(mapper.resolve("gpt-4"), "claude-sonnet-4-5-20250514");

        // 非别名应返回原值
        assert_eq!(mapper.resolve("gemini-2.5-flash"), "gemini-2.5-flash");
    }

    #[test]
    fn test_remove_alias() {
        let mut mapper = ModelMapper::new();
        mapper.add_alias("gpt-4", "claude-sonnet-4-5-20250514");

        let removed = mapper.remove_alias("gpt-4");
        assert_eq!(removed, Some("claude-sonnet-4-5-20250514".to_string()));
        assert!(!mapper.has_alias("gpt-4"));
    }

    #[test]
    fn test_available_models() {
        let mut mapper = ModelMapper::new();
        mapper.add_alias("gpt-4", "claude-sonnet-4-5-20250514");

        let actual_models = vec!["claude-sonnet-4-5-20250514".to_string()];
        let models = mapper.available_models(&actual_models);

        assert_eq!(models.len(), 2);

        let actual = models.iter().find(|m| !m.is_alias).unwrap();
        assert_eq!(actual.id, "claude-sonnet-4-5-20250514");

        let alias = models.iter().find(|m| m.is_alias).unwrap();
        assert_eq!(alias.id, "gpt-4");
        assert_eq!(
            alias.actual_model,
            Some("claude-sonnet-4-5-20250514".to_string())
        );
    }
}
