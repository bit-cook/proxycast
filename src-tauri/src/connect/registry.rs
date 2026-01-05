//! 中转商注册表管理模块
//!
//! 负责从 GitHub 加载和管理中转商注册表，提供中转商信息查询功能。
//!
//! ## 功能
//!
//! - 从远程 GitHub 仓库加载注册表
//! - 本地缓存支持离线访问
//! - 中转商信息查询和验证
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use proxycast_lib::connect::registry::{RelayRegistry, RelayInfo};
//!
//! let registry = RelayRegistry::new(cache_path);
//! registry.load_from_remote().await?;
//!
//! if let Some(info) = registry.get("example-relay") {
//!     println!("中转商: {}", info.name);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use thiserror::Error;

/// 注册表远程 URL
const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/aiclientproxy/connect/main/dist/registry.json";

/// 注册表错误类型
#[derive(Debug, Error)]
pub enum RegistryError {
    /// 网络请求失败
    #[error("网络请求失败: {0}")]
    NetworkError(String),

    /// JSON 解析失败
    #[error("JSON 解析失败: {0}")]
    ParseError(String),

    /// 文件 IO 错误
    #[error("文件操作失败: {0}")]
    IoError(#[from] std::io::Error),

    /// 缓存不存在
    #[error("缓存不存在")]
    NoCacheError,
}

/// 注册表数据结构（JSON 根对象）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryData {
    /// 注册表版本
    pub version: String,
    /// 更新时间
    pub updated_at: String,
    /// 中转商列表
    pub providers: Vec<RelayInfo>,
}

/// 中转商信息
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RelayInfo {
    /// 中转商唯一 ID
    pub id: String,
    /// 中转商名称
    pub name: String,
    /// 中转商描述
    pub description: String,
    /// 品牌信息
    pub branding: RelayBranding,
    /// 相关链接
    pub links: RelayLinks,
    /// API 配置
    pub api: RelayApi,
    /// 联系方式
    pub contact: RelayContact,
    /// 功能特性（可选）
    #[serde(default)]
    pub features: RelayFeatures,
    /// Webhook 配置（可选）
    #[serde(default)]
    pub webhook: Option<RelayWebhook>,
}

/// 品牌信息
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RelayBranding {
    /// Logo URL
    pub logo: String,
    /// 主题色（默认 #6366f1）
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_color() -> String {
    "#6366f1".to_string()
}

/// 相关链接
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RelayLinks {
    /// 主页
    pub homepage: String,
    /// 注册链接（可选）
    #[serde(default)]
    pub register: Option<String>,
    /// 充值链接（可选）
    #[serde(default)]
    pub recharge: Option<String>,
    /// 文档链接（可选）
    #[serde(default)]
    pub docs: Option<String>,
    /// 状态页链接（可选）
    #[serde(default)]
    pub status: Option<String>,
}

/// API 配置
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RelayApi {
    /// API 基础 URL
    pub base_url: String,
    /// 协议类型（如 openai, claude）
    pub protocol: String,
    /// 认证头名称（默认 Authorization）
    #[serde(default = "default_auth_header")]
    pub auth_header: String,
    /// 认证前缀（默认 Bearer）
    #[serde(default = "default_auth_prefix")]
    pub auth_prefix: String,
}

fn default_auth_header() -> String {
    "Authorization".to_string()
}

fn default_auth_prefix() -> String {
    "Bearer".to_string()
}

/// 联系方式
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RelayContact {
    /// 邮箱
    #[serde(default)]
    pub email: Option<String>,
    /// Discord
    #[serde(default)]
    pub discord: Option<String>,
    /// Telegram
    #[serde(default)]
    pub telegram: Option<String>,
    /// Twitter
    #[serde(default)]
    pub twitter: Option<String>,
}

/// 功能特性
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RelayFeatures {
    /// 支持的模型列表
    #[serde(default)]
    pub models: Vec<String>,
    /// 是否支持流式响应
    #[serde(default)]
    pub streaming: bool,
    /// 是否支持函数调用
    #[serde(default)]
    pub function_calling: bool,
    /// 是否支持视觉模型
    #[serde(default)]
    pub vision: bool,
}

/// Webhook 配置
///
/// 用于统计回调，让中转商追踪推广效果。
/// 中转商通过检查 key_prefix 是否为自己下发的 Key 来验证请求。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RelayWebhook {
    /// 回调地址（必须 HTTPS）
    #[serde(default)]
    pub callback_url: Option<String>,
}

/// 中转商注册表管理器
pub struct RelayRegistry {
    /// 中转商数据（ID -> RelayInfo）
    providers: RwLock<HashMap<String, RelayInfo>>,
    /// 缓存文件路径
    cache_path: PathBuf,
}

impl RelayRegistry {
    /// 创建新的注册表实例
    ///
    /// # 参数
    ///
    /// * `cache_path` - 缓存文件路径
    pub fn new(cache_path: PathBuf) -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            cache_path,
        }
    }

    /// 从远程 GitHub 加载注册表
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 加载成功
    /// * `Err(RegistryError)` - 加载失败
    pub async fn load_from_remote(&self) -> Result<(), RegistryError> {
        tracing::info!("从远程加载中转商注册表: {}", REGISTRY_URL);

        let response = reqwest::get(REGISTRY_URL)
            .await
            .map_err(|e| RegistryError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::NetworkError(format!(
                "HTTP 状态码: {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| RegistryError::NetworkError(e.to_string()))?;

        let registry_data: RegistryData =
            serde_json::from_str(&text).map_err(|e| RegistryError::ParseError(e.to_string()))?;

        // 更新内存中的数据
        let mut providers = self
            .providers
            .write()
            .map_err(|_| RegistryError::ParseError("获取写锁失败".to_string()))?;

        providers.clear();
        for provider in registry_data.providers {
            providers.insert(provider.id.clone(), provider);
        }

        tracing::info!("成功加载 {} 个中转商", providers.len());

        // 保存到缓存
        drop(providers); // 释放写锁
        self.save_to_cache()?;

        Ok(())
    }

    /// 从本地缓存加载注册表
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 加载成功
    /// * `Err(RegistryError)` - 加载失败
    pub fn load_from_cache(&self) -> Result<(), RegistryError> {
        if !self.cache_path.exists() {
            return Err(RegistryError::NoCacheError);
        }

        tracing::info!("从缓存加载中转商注册表: {:?}", self.cache_path);

        let content = std::fs::read_to_string(&self.cache_path)?;

        let registry_data: RegistryData =
            serde_json::from_str(&content).map_err(|e| RegistryError::ParseError(e.to_string()))?;

        let mut providers = self
            .providers
            .write()
            .map_err(|_| RegistryError::ParseError("获取写锁失败".to_string()))?;

        providers.clear();
        for provider in registry_data.providers {
            providers.insert(provider.id.clone(), provider);
        }

        tracing::info!("从缓存加载 {} 个中转商", providers.len());

        Ok(())
    }

    /// 保存注册表到本地缓存
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 保存成功
    /// * `Err(RegistryError)` - 保存失败
    pub fn save_to_cache(&self) -> Result<(), RegistryError> {
        let providers = self
            .providers
            .read()
            .map_err(|_| RegistryError::ParseError("获取读锁失败".to_string()))?;

        let registry_data = RegistryData {
            version: "1.0.0".to_string(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            providers: providers.values().cloned().collect(),
        };

        // 确保父目录存在
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&registry_data)
            .map_err(|e| RegistryError::ParseError(e.to_string()))?;

        std::fs::write(&self.cache_path, content)?;

        tracing::info!("注册表已缓存到: {:?}", self.cache_path);

        Ok(())
    }

    /// 查询中转商信息
    ///
    /// # 参数
    ///
    /// * `id` - 中转商 ID
    ///
    /// # 返回值
    ///
    /// * `Some(RelayInfo)` - 找到对应的中转商
    /// * `None` - 未找到
    pub fn get(&self, id: &str) -> Option<RelayInfo> {
        self.providers
            .read()
            .ok()
            .and_then(|providers| providers.get(id).cloned())
    }

    /// 验证中转商是否存在于注册表中
    ///
    /// # 参数
    ///
    /// * `id` - 中转商 ID
    ///
    /// # 返回值
    ///
    /// * `true` - 存在
    /// * `false` - 不存在
    pub fn is_valid(&self, id: &str) -> bool {
        self.providers
            .read()
            .ok()
            .map(|providers| providers.contains_key(id))
            .unwrap_or(false)
    }

    /// 获取所有中转商列表
    ///
    /// # 返回值
    ///
    /// 所有中转商信息的列表
    pub fn list(&self) -> Vec<RelayInfo> {
        self.providers
            .read()
            .ok()
            .map(|providers| providers.values().cloned().collect())
            .unwrap_or_default()
    }

    /// 获取中转商数量
    pub fn len(&self) -> usize {
        self.providers
            .read()
            .ok()
            .map(|providers| providers.len())
            .unwrap_or(0)
    }

    /// 检查注册表是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 直接从 RegistryData 加载（用于测试）
    #[cfg(test)]
    pub fn load_from_data(&self, data: RegistryData) {
        if let Ok(mut providers) = self.providers.write() {
            providers.clear();
            for provider in data.providers {
                providers.insert(provider.id.clone(), provider);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// 创建测试用的 RelayInfo
    fn create_test_relay_info(id: &str, name: &str) -> RelayInfo {
        RelayInfo {
            id: id.to_string(),
            name: name.to_string(),
            description: format!("{} 描述", name),
            branding: RelayBranding {
                logo: format!("https://example.com/{}/logo.png", id),
                color: "#6366f1".to_string(),
            },
            links: RelayLinks {
                homepage: format!("https://{}.example.com", id),
                register: Some(format!("https://{}.example.com/register", id)),
                recharge: None,
                docs: Some(format!("https://docs.{}.example.com", id)),
                status: None,
            },
            api: RelayApi {
                base_url: format!("https://api.{}.example.com/v1", id),
                protocol: "openai".to_string(),
                auth_header: "Authorization".to_string(),
                auth_prefix: "Bearer".to_string(),
            },
            contact: RelayContact {
                email: Some(format!("support@{}.example.com", id)),
                discord: None,
                telegram: None,
                twitter: None,
            },
            features: RelayFeatures::default(),
            webhook: None,
        }
    }

    /// 创建测试用的 RegistryData
    fn create_test_registry_data(providers: Vec<RelayInfo>) -> RegistryData {
        RegistryData {
            version: "1.0.0".to_string(),
            updated_at: "2026-01-05T00:00:00Z".to_string(),
            providers,
        }
    }

    #[test]
    fn test_registry_new() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("registry.json");
        let registry = RelayRegistry::new(cache_path.clone());

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_load_from_data() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("registry.json");
        let registry = RelayRegistry::new(cache_path);

        let relay1 = create_test_relay_info("relay-1", "中转站 1");
        let relay2 = create_test_relay_info("relay-2", "中转站 2");
        let data = create_test_registry_data(vec![relay1.clone(), relay2.clone()]);

        registry.load_from_data(data);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_get_existing() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("registry.json");
        let registry = RelayRegistry::new(cache_path);

        let relay = create_test_relay_info("test-relay", "测试中转站");
        let data = create_test_registry_data(vec![relay.clone()]);
        registry.load_from_data(data);

        let result = registry.get("test-relay");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "测试中转站");
    }

    #[test]
    fn test_registry_get_non_existing() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("registry.json");
        let registry = RelayRegistry::new(cache_path);

        let relay = create_test_relay_info("test-relay", "测试中转站");
        let data = create_test_registry_data(vec![relay]);
        registry.load_from_data(data);

        let result = registry.get("non-existing");
        assert!(result.is_none());
    }

    #[test]
    fn test_registry_is_valid() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("registry.json");
        let registry = RelayRegistry::new(cache_path);

        let relay = create_test_relay_info("valid-relay", "有效中转站");
        let data = create_test_registry_data(vec![relay]);
        registry.load_from_data(data);

        assert!(registry.is_valid("valid-relay"));
        assert!(!registry.is_valid("invalid-relay"));
    }

    #[test]
    fn test_registry_list() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("registry.json");
        let registry = RelayRegistry::new(cache_path);

        let relay1 = create_test_relay_info("relay-1", "中转站 1");
        let relay2 = create_test_relay_info("relay-2", "中转站 2");
        let data = create_test_registry_data(vec![relay1, relay2]);
        registry.load_from_data(data);

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_registry_cache_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("registry.json");

        // 创建并保存
        let registry1 = RelayRegistry::new(cache_path.clone());
        let relay = create_test_relay_info("cached-relay", "缓存中转站");
        let data = create_test_registry_data(vec![relay.clone()]);
        registry1.load_from_data(data);
        registry1.save_to_cache().unwrap();

        // 从缓存加载
        let registry2 = RelayRegistry::new(cache_path);
        registry2.load_from_cache().unwrap();

        assert_eq!(registry2.len(), 1);
        let loaded = registry2.get("cached-relay").unwrap();
        assert_eq!(loaded.name, relay.name);
        assert_eq!(loaded.description, relay.description);
    }

    #[test]
    fn test_registry_load_from_cache_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("non_existing.json");
        let registry = RelayRegistry::new(cache_path);

        let result = registry.load_from_cache();
        assert!(matches!(result, Err(RegistryError::NoCacheError)));
    }

    #[test]
    fn test_relay_info_serialization() {
        let relay = create_test_relay_info("test", "测试");
        let json = serde_json::to_string(&relay).unwrap();
        let deserialized: RelayInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(relay, deserialized);
    }

    #[test]
    fn test_relay_branding_default_color() {
        let json = r#"{"logo": "https://example.com/logo.png"}"#;
        let branding: RelayBranding = serde_json::from_str(json).unwrap();

        assert_eq!(branding.color, "#6366f1");
    }

    #[test]
    fn test_relay_api_default_auth() {
        let json = r#"{"base_url": "https://api.example.com", "protocol": "openai"}"#;
        let api: RelayApi = serde_json::from_str(json).unwrap();

        assert_eq!(api.auth_header, "Authorization");
        assert_eq!(api.auth_prefix, "Bearer");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    /// 生成有效的中转商 ID
    fn arb_relay_id() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,20}[a-z0-9]"
            .prop_filter("非空且有效", |s| !s.is_empty() && s.len() >= 2)
    }

    /// 生成有效的中转商名称
    fn arb_relay_name() -> impl Strategy<Value = String> {
        "[a-zA-Z\u{4e00}-\u{9fff}]{1,20}".prop_filter("非空", |s| !s.is_empty())
    }

    /// 生成测试用的 RelayInfo
    fn arb_relay_info() -> impl Strategy<Value = RelayInfo> {
        (arb_relay_id(), arb_relay_name()).prop_map(|(id, name)| RelayInfo {
            id: id.clone(),
            name: name.clone(),
            description: format!("{} 描述", name),
            branding: RelayBranding {
                logo: format!("https://example.com/{}/logo.png", id),
                color: "#6366f1".to_string(),
            },
            links: RelayLinks {
                homepage: format!("https://{}.example.com", id),
                register: None,
                recharge: None,
                docs: None,
                status: None,
            },
            api: RelayApi {
                base_url: format!("https://api.{}.example.com/v1", id),
                protocol: "openai".to_string(),
                auth_header: "Authorization".to_string(),
                auth_prefix: "Bearer".to_string(),
            },
            contact: RelayContact {
                email: None,
                discord: None,
                telegram: None,
                twitter: None,
            },
            features: RelayFeatures::default(),
            webhook: None,
        })
    }

    /// 生成多个不重复 ID 的 RelayInfo 列表
    fn arb_relay_info_list(max_size: usize) -> impl Strategy<Value = Vec<RelayInfo>> {
        prop::collection::vec(arb_relay_info(), 0..=max_size).prop_map(|relays| {
            // 去重：保留每个 ID 的第一个出现
            let mut seen = std::collections::HashSet::new();
            relays
                .into_iter()
                .filter(|r| seen.insert(r.id.clone()))
                .collect()
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: proxycast-connect, Property 3: Registry Lookup Consistency
        /// Validates: Requirements 2.3, 2.4
        ///
        /// *For any* RelayRegistry and relay ID, if the ID exists in the registry
        /// then `get(id)` SHALL return the corresponding RelayInfo, and if the ID
        /// does not exist then `get(id)` SHALL return None.
        #[test]
        fn prop_registry_lookup_consistency(
            relays in arb_relay_info_list(10),
            query_id in arb_relay_id(),
        ) {
            let temp_dir = TempDir::new().unwrap();
            let cache_path = temp_dir.path().join("registry.json");
            let registry = RelayRegistry::new(cache_path);

            // 加载测试数据
            let data = RegistryData {
                version: "1.0.0".to_string(),
                updated_at: "2026-01-05T00:00:00Z".to_string(),
                providers: relays.clone(),
            };
            registry.load_from_data(data);

            // 查找是否存在于原始列表中
            let expected = relays.iter().find(|r| r.id == query_id);

            // 执行查询
            let result = registry.get(&query_id);

            // 验证一致性
            match (expected, result) {
                (Some(expected_info), Some(result_info)) => {
                    // ID 存在时，返回的信息应该匹配
                    prop_assert_eq!(
                        &result_info.id, &expected_info.id,
                        "ID 不匹配"
                    );
                    prop_assert_eq!(
                        &result_info.name, &expected_info.name,
                        "名称不匹配"
                    );
                    prop_assert_eq!(
                        &result_info.description, &expected_info.description,
                        "描述不匹配"
                    );
                }
                (None, None) => {
                    // ID 不存在时，返回 None
                }
                (Some(_), None) => {
                    prop_assert!(false, "期望找到 ID {} 但返回 None", query_id);
                }
                (None, Some(_)) => {
                    prop_assert!(false, "ID {} 不应该存在但返回了结果", query_id);
                }
            }

            // 验证 is_valid 与 get 的一致性
            let is_valid = registry.is_valid(&query_id);
            let get_result = registry.get(&query_id);
            prop_assert_eq!(
                is_valid, get_result.is_some(),
                "is_valid 与 get 结果不一致"
            );
        }

        /// 测试缓存往返一致性
        #[test]
        fn prop_cache_round_trip(relays in arb_relay_info_list(5)) {
            let temp_dir = TempDir::new().unwrap();
            let cache_path = temp_dir.path().join("registry.json");

            // 创建并保存
            let registry1 = RelayRegistry::new(cache_path.clone());
            let data = RegistryData {
                version: "1.0.0".to_string(),
                updated_at: "2026-01-05T00:00:00Z".to_string(),
                providers: relays.clone(),
            };
            registry1.load_from_data(data);
            registry1.save_to_cache().unwrap();

            // 从缓存加载
            let registry2 = RelayRegistry::new(cache_path);
            registry2.load_from_cache().unwrap();

            // 验证数量一致
            prop_assert_eq!(
                registry1.len(), registry2.len(),
                "缓存往返后数量不一致"
            );

            // 验证每个 relay 都能正确加载
            for relay in &relays {
                let loaded = registry2.get(&relay.id);
                prop_assert!(
                    loaded.is_some(),
                    "缓存往返后找不到 ID: {}", relay.id
                );
                let loaded = loaded.unwrap();
                prop_assert_eq!(
                    &loaded.name, &relay.name,
                    "缓存往返后名称不匹配"
                );
            }
        }

        /// 测试 list 返回所有已加载的中转商
        #[test]
        fn prop_list_returns_all(relays in arb_relay_info_list(10)) {
            let temp_dir = TempDir::new().unwrap();
            let cache_path = temp_dir.path().join("registry.json");
            let registry = RelayRegistry::new(cache_path);

            let data = RegistryData {
                version: "1.0.0".to_string(),
                updated_at: "2026-01-05T00:00:00Z".to_string(),
                providers: relays.clone(),
            };
            registry.load_from_data(data);

            let list = registry.list();

            // 验证数量
            prop_assert_eq!(
                list.len(), relays.len(),
                "list 返回数量不正确"
            );

            // 验证每个 relay 都在列表中
            for relay in &relays {
                prop_assert!(
                    list.iter().any(|r| r.id == relay.id),
                    "list 中缺少 ID: {}", relay.id
                );
            }
        }
    }
}
