//! API Key Provider 管理服务
//!
//! 提供 API Key Provider 的 CRUD 操作、加密存储和轮询负载均衡功能。
//!
//! **Feature: provider-ui-refactor**
//! **Validates: Requirements 7.3, 9.1, 9.2, 9.3**

use crate::database::dao::api_key_provider::{
    ApiKeyEntry, ApiKeyProvider, ApiKeyProviderDao, ApiProviderType, ProviderGroup,
    ProviderWithKeys,
};
use crate::database::system_providers::{get_system_providers, to_api_key_provider};
use crate::database::DbConnection;
use crate::models::{CredentialData, CredentialSource, PoolProviderType, ProviderCredential};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

// ============================================================================
// 连接测试结果
// ============================================================================

/// 连接测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    /// 是否成功
    pub success: bool,
    /// 延迟（毫秒）
    pub latency_ms: Option<u64>,
    /// 错误信息
    pub error: Option<String>,
    /// 模型列表（如果使用 models 端点测试）
    pub models: Option<Vec<String>>,
}

// ============================================================================
// 加密服务
// ============================================================================

/// 简单的 API Key 加密服务
/// 使用 XOR 加密 + Base64 编码
/// 注意：这是一个简单的混淆方案，不是强加密
struct EncryptionService {
    /// 加密密钥（从机器 ID 派生）
    key: Vec<u8>,
}

impl EncryptionService {
    /// 创建新的加密服务
    fn new() -> Self {
        // 使用机器特定信息生成密钥
        let machine_id = Self::get_machine_id();
        let mut hasher = Sha256::new();
        hasher.update(machine_id.as_bytes());
        hasher.update(b"proxycast-api-key-encryption-salt");
        let key = hasher.finalize().to_vec();

        Self { key }
    }

    /// 获取机器 ID
    fn get_machine_id() -> String {
        // 尝试获取机器 ID，失败则使用默认值
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            return id.trim().to_string();
        }
        if let Ok(id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
            return id.trim().to_string();
        }
        // macOS: 使用 IOPlatformUUID
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("ioreg")
                .args(["-rd1", "-c", "IOPlatformExpertDevice"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("IOPlatformUUID") {
                        if let Some(uuid) = line.split('"').nth(3) {
                            return uuid.to_string();
                        }
                    }
                }
            }
        }
        // 默认值
        "proxycast-default-machine-id".to_string()
    }

    /// 加密 API Key
    fn encrypt(&self, plaintext: &str) -> String {
        let encrypted: Vec<u8> = plaintext
            .as_bytes()
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ self.key[i % self.key.len()])
            .collect();
        BASE64.encode(encrypted)
    }

    /// 解密 API Key
    fn decrypt(&self, ciphertext: &str) -> Result<String, String> {
        let encrypted = BASE64
            .decode(ciphertext)
            .map_err(|e| format!("Base64 解码失败: {}", e))?;
        let decrypted: Vec<u8> = encrypted
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ self.key[i % self.key.len()])
            .collect();
        String::from_utf8(decrypted).map_err(|e| format!("UTF-8 解码失败: {}", e))
    }

    /// 检查是否为加密后的值（非明文）
    fn is_encrypted(&self, value: &str) -> bool {
        // 加密后的值是 Base64 编码的，通常不包含常见的 API Key 前缀
        !value.starts_with("sk-")
            && !value.starts_with("pk-")
            && !value.starts_with("api-")
            && BASE64.decode(value).is_ok()
    }
}

// ============================================================================
// API Key Provider 服务
// ============================================================================

/// API Key Provider 管理服务
pub struct ApiKeyProviderService {
    /// 加密服务
    encryption: EncryptionService,
    /// 轮询索引（按 provider_id 分组）
    round_robin_index: RwLock<HashMap<String, AtomicUsize>>,
}

impl Default for ApiKeyProviderService {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyProviderService {
    /// 创建新的服务实例
    pub fn new() -> Self {
        Self {
            encryption: EncryptionService::new(),
            round_robin_index: RwLock::new(HashMap::new()),
        }
    }

    // ==================== Provider 操作 ====================

    /// 初始化系统 Provider
    /// 检查数据库中是否存在系统 Provider，如果不存在则插入
    /// **Validates: Requirements 9.3**
    pub fn initialize_system_providers(&self, db: &DbConnection) -> Result<usize, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let system_providers = get_system_providers();
        let mut inserted_count = 0;

        for def in &system_providers {
            // 检查是否已存在
            let existing =
                ApiKeyProviderDao::get_provider_by_id(&conn, def.id).map_err(|e| e.to_string())?;

            if existing.is_none() {
                // 插入新的系统 Provider
                let provider = to_api_key_provider(def);
                ApiKeyProviderDao::insert_provider(&conn, &provider).map_err(|e| e.to_string())?;
                inserted_count += 1;
            }
        }

        if inserted_count > 0 {
            tracing::info!("初始化了 {} 个系统 Provider", inserted_count);
        }

        Ok(inserted_count)
    }

    /// 获取所有 Provider（包含 API Keys）
    /// 首次调用时会自动初始化系统 Provider
    pub fn get_all_providers(&self, db: &DbConnection) -> Result<Vec<ProviderWithKeys>, String> {
        // 首先确保系统 Provider 已初始化
        self.initialize_system_providers(db)?;

        let conn = db.lock().map_err(|e| e.to_string())?;
        let providers =
            ApiKeyProviderDao::get_all_providers_with_keys(&conn).map_err(|e| e.to_string())?;

        tracing::debug!(
            "[ApiKeyProviderService] 获取到 {} 个 Provider",
            providers.len()
        );

        for p in &providers {
            tracing::debug!(
                "[ApiKeyProviderService] Provider: id={}, name={}, api_keys={}",
                p.provider.id,
                p.provider.name,
                p.api_keys.len()
            );
        }

        Ok(providers)
    }

    /// 获取单个 Provider（包含 API Keys）
    pub fn get_provider(
        &self,
        db: &DbConnection,
        id: &str,
    ) -> Result<Option<ProviderWithKeys>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let provider =
            ApiKeyProviderDao::get_provider_by_id(&conn, id).map_err(|e| e.to_string())?;

        match provider {
            Some(p) => {
                let api_keys = ApiKeyProviderDao::get_api_keys_by_provider(&conn, id)
                    .map_err(|e| e.to_string())?;
                Ok(Some(ProviderWithKeys {
                    provider: p,
                    api_keys,
                }))
            }
            None => Ok(None),
        }
    }

    /// 添加自定义 Provider
    pub fn add_custom_provider(
        &self,
        db: &DbConnection,
        name: String,
        provider_type: ApiProviderType,
        api_host: String,
        api_version: Option<String>,
        project: Option<String>,
        location: Option<String>,
        region: Option<String>,
    ) -> Result<ApiKeyProvider, String> {
        let now = Utc::now();
        let id = format!("custom-{}", uuid::Uuid::new_v4());

        let provider = ApiKeyProvider {
            id: id.clone(),
            name,
            provider_type,
            api_host,
            is_system: false,
            group: ProviderGroup::Custom,
            enabled: true,
            sort_order: 9999, // 自定义 Provider 排在最后
            api_version,
            project,
            location,
            region,
            custom_models: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        let conn = db.lock().map_err(|e| e.to_string())?;
        ApiKeyProviderDao::insert_provider(&conn, &provider).map_err(|e| e.to_string())?;

        Ok(provider)
    }

    /// 更新 Provider 配置
    pub fn update_provider(
        &self,
        db: &DbConnection,
        id: &str,
        name: Option<String>,
        provider_type: Option<ApiProviderType>,
        api_host: Option<String>,
        enabled: Option<bool>,
        sort_order: Option<i32>,
        api_version: Option<String>,
        project: Option<String>,
        location: Option<String>,
        region: Option<String>,
        custom_models: Option<Vec<String>>,
    ) -> Result<ApiKeyProvider, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let mut provider = ApiKeyProviderDao::get_provider_by_id(&conn, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Provider not found: {}", id))?;

        // 更新字段
        if let Some(n) = name {
            provider.name = n;
        }
        // 只有自定义 Provider 才能修改类型
        if let Some(t) = provider_type {
            if provider.is_system {
                return Err("系统 Provider 不允许修改类型".to_string());
            }
            provider.provider_type = t;
        }
        if let Some(h) = api_host {
            provider.api_host = h;
        }
        if let Some(e) = enabled {
            provider.enabled = e;
        }
        if let Some(s) = sort_order {
            provider.sort_order = s;
        }
        if let Some(v) = api_version {
            provider.api_version = if v.is_empty() { None } else { Some(v) };
        }
        if let Some(p) = project {
            provider.project = if p.is_empty() { None } else { Some(p) };
        }
        if let Some(l) = location {
            provider.location = if l.is_empty() { None } else { Some(l) };
        }
        if let Some(r) = region {
            provider.region = if r.is_empty() { None } else { Some(r) };
        }
        if let Some(models) = custom_models {
            provider.custom_models = models;
        }
        provider.updated_at = Utc::now();

        ApiKeyProviderDao::update_provider(&conn, &provider).map_err(|e| e.to_string())?;

        Ok(provider)
    }

    /// 删除自定义 Provider
    /// 系统 Provider 不允许删除
    pub fn delete_custom_provider(&self, db: &DbConnection, id: &str) -> Result<bool, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // 检查是否为系统 Provider
        let provider = ApiKeyProviderDao::get_provider_by_id(&conn, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Provider not found: {}", id))?;

        if provider.is_system {
            return Err("不允许删除系统 Provider".to_string());
        }

        ApiKeyProviderDao::delete_provider(&conn, id).map_err(|e| e.to_string())
    }

    // ==================== API Key 操作 ====================

    /// 添加 API Key
    ///
    /// 当添加第一个 API Key 时，会自动启用 Provider
    /// 使用数据库事务确保操作的原子性
    pub fn add_api_key(
        &self,
        db: &DbConnection,
        provider_id: &str,
        api_key: &str,
        alias: Option<String>,
    ) -> Result<ApiKeyEntry, String> {
        tracing::info!(
            "[ApiKeyProviderService] 开始添加 API Key: provider_id={}",
            provider_id
        );

        let mut conn = db.lock().map_err(|e| e.to_string())?;

        // 使用事务确保操作的原子性
        let tx = conn
            .transaction()
            .map_err(|e| format!("开始事务失败: {}", e))?;

        // 验证 Provider 存在
        let provider = ApiKeyProviderDao::get_provider_by_id(&tx, provider_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

        tracing::info!(
            "[ApiKeyProviderService] 找到 Provider: name={}, id={}",
            provider.name,
            provider.id
        );

        // 检查 API Key 是否已存在（防重复添加）
        let existing_keys = ApiKeyProviderDao::get_api_keys_by_provider(&tx, provider_id)
            .map_err(|e| e.to_string())?;

        tracing::info!(
            "[ApiKeyProviderService] 当前已有 {} 个 API Key",
            existing_keys.len()
        );

        // 检查是否有相同的 API Key（比较加密后的值）
        let encrypted_input = self.encryption.encrypt(api_key);
        for existing_key in &existing_keys {
            if existing_key.api_key_encrypted == encrypted_input {
                return Err("该 API Key 已存在".to_string());
            }
        }

        let should_enable_provider = existing_keys.is_empty() && !provider.enabled;

        let now = Utc::now();
        let key = ApiKeyEntry {
            id: uuid::Uuid::new_v4().to_string(),
            provider_id: provider_id.to_string(),
            api_key_encrypted: encrypted_input,
            alias: alias.clone(),
            enabled: true,
            usage_count: 0,
            error_count: 0,
            last_used_at: None,
            created_at: now,
        };

        // 插入 API Key
        ApiKeyProviderDao::insert_api_key(&tx, &key).map_err(|e| e.to_string())?;

        tracing::info!(
            "[ApiKeyProviderService] API Key 已插入: id={}, provider_id={}",
            key.id,
            key.provider_id
        );

        // 如果是第一个 API Key，自动启用 Provider
        if should_enable_provider {
            let mut updated_provider = provider;
            updated_provider.enabled = true;
            updated_provider.updated_at = now;
            ApiKeyProviderDao::update_provider(&tx, &updated_provider)
                .map_err(|e| e.to_string())?;
            tracing::info!(
                "[ApiKeyProviderService] 自动启用 Provider: {} (添加了第一个 API Key)",
                provider_id
            );
        }

        // 提交事务
        tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;

        tracing::info!(
            "[ApiKeyProviderService] 成功添加 API Key: provider={}, alias={:?}",
            provider_id,
            alias
        );

        Ok(key)
    }

    /// 删除 API Key
    pub fn delete_api_key(&self, db: &DbConnection, key_id: &str) -> Result<bool, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ApiKeyProviderDao::delete_api_key(&conn, key_id).map_err(|e| e.to_string())
    }

    /// 切换 API Key 启用状态
    pub fn toggle_api_key(
        &self,
        db: &DbConnection,
        key_id: &str,
        enabled: bool,
    ) -> Result<ApiKeyEntry, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let mut key = ApiKeyProviderDao::get_api_key_by_id(&conn, key_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("API Key not found: {}", key_id))?;

        key.enabled = enabled;
        ApiKeyProviderDao::update_api_key(&conn, &key).map_err(|e| e.to_string())?;

        Ok(key)
    }

    /// 更新 API Key 别名
    pub fn update_api_key_alias(
        &self,
        db: &DbConnection,
        key_id: &str,
        alias: Option<String>,
    ) -> Result<ApiKeyEntry, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let mut key = ApiKeyProviderDao::get_api_key_by_id(&conn, key_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("API Key not found: {}", key_id))?;

        key.alias = alias;
        ApiKeyProviderDao::update_api_key(&conn, &key).map_err(|e| e.to_string())?;

        Ok(key)
    }

    // ==================== 轮询负载均衡 ====================

    /// 获取下一个可用的 API Key（轮询负载均衡）
    /// **Validates: Requirements 7.3**
    pub fn get_next_api_key(
        &self,
        db: &DbConnection,
        provider_id: &str,
    ) -> Result<Option<String>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // 获取所有启用的 API Keys
        let keys = ApiKeyProviderDao::get_enabled_api_keys_by_provider(&conn, provider_id)
            .map_err(|e| e.to_string())?;

        if keys.is_empty() {
            return Ok(None);
        }

        // 获取或创建轮询索引
        let index = {
            let mut indices = self.round_robin_index.write().map_err(|e| e.to_string())?;
            indices
                .entry(provider_id.to_string())
                .or_insert_with(|| AtomicUsize::new(0))
                .fetch_add(1, Ordering::SeqCst)
        };

        // 选择 API Key
        let selected_key = &keys[index % keys.len()];

        // 解密并返回
        let decrypted = self.encryption.decrypt(&selected_key.api_key_encrypted)?;
        Ok(Some(decrypted))
    }

    /// 获取下一个可用的 API Key 条目（包含 ID，用于记录使用）
    pub fn get_next_api_key_entry(
        &self,
        db: &DbConnection,
        provider_id: &str,
    ) -> Result<Option<(String, String)>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // 获取所有启用的 API Keys
        let keys = ApiKeyProviderDao::get_enabled_api_keys_by_provider(&conn, provider_id)
            .map_err(|e| e.to_string())?;

        if keys.is_empty() {
            return Ok(None);
        }

        // 获取或创建轮询索引
        let index = {
            let mut indices = self.round_robin_index.write().map_err(|e| e.to_string())?;
            indices
                .entry(provider_id.to_string())
                .or_insert_with(|| AtomicUsize::new(0))
                .fetch_add(1, Ordering::SeqCst)
        };

        // 选择 API Key
        let selected_key = &keys[index % keys.len()];

        // 解密并返回
        let decrypted = self.encryption.decrypt(&selected_key.api_key_encrypted)?;
        Ok(Some((selected_key.id.clone(), decrypted)))
    }

    /// 记录 API Key 使用
    pub fn record_usage(&self, db: &DbConnection, key_id: &str) -> Result<(), String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let key = ApiKeyProviderDao::get_api_key_by_id(&conn, key_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("API Key not found: {}", key_id))?;

        ApiKeyProviderDao::update_api_key_usage(&conn, key_id, key.usage_count + 1, Utc::now())
            .map_err(|e| e.to_string())
    }

    /// 获取下一个可用的 API Key 以及 Provider 信息（按 provider_id 精确查找）
    /// 用于支持 X-Provider-Id 请求头指定具体的 Provider
    pub fn get_next_api_key_with_provider_info(
        &self,
        db: &DbConnection,
        provider_id: &str,
    ) -> Result<Option<(String, ApiKeyProvider)>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // 获取 Provider 信息
        let provider = match ApiKeyProviderDao::get_provider_by_id(&conn, provider_id)
            .map_err(|e| e.to_string())?
        {
            Some(p) => p,
            None => return Ok(None),
        };

        // 检查 Provider 是否启用
        if !provider.enabled {
            return Ok(None);
        }

        // 获取该 Provider 的所有启用的 API Keys
        let keys = ApiKeyProviderDao::get_enabled_api_keys_by_provider(&conn, provider_id)
            .map_err(|e| e.to_string())?;

        if keys.is_empty() {
            return Ok(None);
        }

        // 获取或创建轮询索引
        let index = {
            let mut indices = self.round_robin_index.write().map_err(|e| e.to_string())?;
            indices
                .entry(provider_id.to_string())
                .or_insert_with(|| AtomicUsize::new(0))
                .fetch_add(1, Ordering::SeqCst)
        };

        // 选择 API Key
        let selected_key = &keys[index % keys.len()];

        // 解密并返回
        let decrypted = self.encryption.decrypt(&selected_key.api_key_encrypted)?;
        Ok(Some((decrypted, provider)))
    }

    /// 按 Provider 类型获取下一个可用的 API Key（轮询负载均衡）
    /// 这个方法会查找所有该类型的 Provider（包括自定义 Provider）
    pub fn get_next_api_key_by_type(
        &self,
        db: &DbConnection,
        provider_type: ApiProviderType,
    ) -> Result<Option<(String, String, ApiKeyProvider)>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // 获取所有启用的 API Keys（按类型）
        let keys = ApiKeyProviderDao::get_enabled_api_keys_by_type(&conn, provider_type)
            .map_err(|e| e.to_string())?;

        if keys.is_empty() {
            return Ok(None);
        }

        // 使用类型名称作为轮询索引的 key
        let type_key = format!("type:{}", provider_type);
        let index = {
            let mut indices = self.round_robin_index.write().map_err(|e| e.to_string())?;
            indices
                .entry(type_key)
                .or_insert_with(|| AtomicUsize::new(0))
                .fetch_add(1, Ordering::SeqCst)
        };

        // 选择 API Key
        let (selected_key, provider) = &keys[index % keys.len()];

        // 解密并返回
        let decrypted = self.encryption.decrypt(&selected_key.api_key_encrypted)?;
        Ok(Some((selected_key.id.clone(), decrypted, provider.clone())))
    }

    /// 记录 API Key 错误
    pub fn record_error(&self, db: &DbConnection, key_id: &str) -> Result<(), String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ApiKeyProviderDao::increment_api_key_error(&conn, key_id).map_err(|e| e.to_string())
    }

    // ==================== 加密相关 ====================

    /// 检查 API Key 是否已加密
    pub fn is_encrypted(&self, value: &str) -> bool {
        self.encryption.is_encrypted(value)
    }

    /// 解密 API Key（用于 API 调用）
    pub fn decrypt_api_key(&self, encrypted: &str) -> Result<String, String> {
        self.encryption.decrypt(encrypted)
    }

    /// 加密 API Key（用于存储）
    pub fn encrypt_api_key(&self, plaintext: &str) -> String {
        self.encryption.encrypt(plaintext)
    }

    // ==================== UI 状态 ====================

    /// 获取 UI 状态
    pub fn get_ui_state(&self, db: &DbConnection, key: &str) -> Result<Option<String>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ApiKeyProviderDao::get_ui_state(&conn, key).map_err(|e| e.to_string())
    }

    /// 设置 UI 状态
    pub fn set_ui_state(&self, db: &DbConnection, key: &str, value: &str) -> Result<(), String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ApiKeyProviderDao::set_ui_state(&conn, key, value).map_err(|e| e.to_string())
    }

    /// 批量更新 Provider 排序顺序
    /// **Validates: Requirements 8.4**
    pub fn update_provider_sort_orders(
        &self,
        db: &DbConnection,
        sort_orders: Vec<(String, i32)>,
    ) -> Result<(), String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ApiKeyProviderDao::update_provider_sort_orders(&conn, &sort_orders)
            .map_err(|e| e.to_string())
    }

    // ==================== 导入导出 ====================

    /// 导出配置
    pub fn export_config(
        &self,
        db: &DbConnection,
        include_keys: bool,
    ) -> Result<serde_json::Value, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let providers =
            ApiKeyProviderDao::get_all_providers_with_keys(&conn).map_err(|e| e.to_string())?;

        let export_data = if include_keys {
            // 包含 API Keys（但不包含实际的 key 值）
            let providers_json: Vec<serde_json::Value> = providers
                .iter()
                .map(|p| {
                    let keys: Vec<serde_json::Value> = p
                        .api_keys
                        .iter()
                        .map(|k| {
                            serde_json::json!({
                                "id": k.id,
                                "alias": k.alias,
                                "enabled": k.enabled,
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "provider": p.provider,
                        "api_keys": keys,
                    })
                })
                .collect();
            serde_json::json!({
                "version": "1.0",
                "exported_at": Utc::now().to_rfc3339(),
                "providers": providers_json,
            })
        } else {
            // 不包含 API Keys
            let providers_json: Vec<serde_json::Value> = providers
                .iter()
                .map(|p| serde_json::json!(p.provider))
                .collect();
            serde_json::json!({
                "version": "1.0",
                "exported_at": Utc::now().to_rfc3339(),
                "providers": providers_json,
            })
        };

        Ok(export_data)
    }

    /// 导入配置
    pub fn import_config(
        &self,
        db: &DbConnection,
        config_json: &str,
    ) -> Result<ImportResult, String> {
        let config: serde_json::Value =
            serde_json::from_str(config_json).map_err(|e| format!("JSON 解析失败: {}", e))?;

        let providers = config["providers"]
            .as_array()
            .ok_or_else(|| "配置格式错误: 缺少 providers 数组".to_string())?;

        let conn = db.lock().map_err(|e| e.to_string())?;
        let mut imported_providers = 0;
        let mut skipped_providers = 0;
        let mut errors = Vec::new();

        for provider_json in providers {
            let provider_data = if provider_json.get("provider").is_some() {
                &provider_json["provider"]
            } else {
                provider_json
            };

            let id = provider_data["id"]
                .as_str()
                .ok_or_else(|| "Provider 缺少 id".to_string())?;

            // 检查是否已存在
            if ApiKeyProviderDao::get_provider_by_id(&conn, id)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                skipped_providers += 1;
                continue;
            }

            // 解析 Provider
            let provider: ApiKeyProvider = serde_json::from_value(provider_data.clone())
                .map_err(|e| format!("Provider 解析失败: {}", e))?;

            // 插入 Provider
            if let Err(e) = ApiKeyProviderDao::insert_provider(&conn, &provider) {
                errors.push(format!("导入 Provider {} 失败: {}", id, e));
                continue;
            }

            imported_providers += 1;
        }

        Ok(ImportResult {
            success: errors.is_empty(),
            imported_providers,
            imported_api_keys: 0, // API Keys 不在导入中包含实际值
            skipped_providers,
            errors,
        })
    }

    // ==================== 智能降级 ====================

    /// 根据 PoolProviderType 获取降级凭证
    ///
    /// 用于智能降级场景：当 Provider Pool 无可用凭证时，自动从 API Key Provider 查找
    ///
    /// 降级策略：
    /// 1. 首先通过类型映射查找 (PoolProviderType → ApiProviderType)
    /// 2. 如果类型映射失败，尝试通过 provider_id 直接查找 (支持 60+ Provider)
    ///
    /// # 参数
    /// - `db`: 数据库连接
    /// - `pool_type`: Provider Pool 中的 Provider 类型
    /// - `provider_id_hint`: 可选的 provider_id 提示，如 "deepseek", "dashscope"
    /// - `client_type`: 客户端类型，用于兼容性检查
    ///
    /// # 返回
    /// - `Ok(Some(credential))`: 找到可用的降级凭证
    /// - `Ok(None)`: 没有找到可用的降级凭证
    /// - `Err(e)`: 查询过程中发生错误
    pub async fn get_fallback_credential(
        &self,
        db: &DbConnection,
        pool_type: &PoolProviderType,
        provider_id_hint: Option<&str>,
        client_type: Option<&crate::server::client_detector::ClientType>,
    ) -> Result<Option<ProviderCredential>, String> {
        eprintln!(
            "[get_fallback_credential] 开始查找: pool_type={:?}, provider_id_hint={:?}",
            pool_type, provider_id_hint
        );

        // 策略 1: 优先通过 provider_id 直接查找 (支持 deepseek, moonshot 等 60+ Provider)
        // 这些 Provider 在 API Key Provider 中有独立配置，应该优先使用
        if let Some(provider_id) = provider_id_hint {
            eprintln!(
                "[get_fallback_credential] 尝试按 provider_id '{}' 查找",
                provider_id
            );
            if let Some(cred) = self
                .find_by_provider_id(db, provider_id, client_type)
                .await?
            {
                eprintln!(
                    "[get_fallback_credential] 通过 provider_id '{}' 找到凭证: {:?}",
                    provider_id, cred.name
                );
                return Ok(Some(cred));
            }
            eprintln!(
                "[get_fallback_credential] provider_id '{}' 未找到凭证",
                provider_id
            );
        }

        // 策略 2: 通过类型映射查找（降级方案）
        if let Some(api_type) = self.map_pool_type_to_api_type(pool_type) {
            eprintln!(
                "[get_fallback_credential] 尝试类型映射: {:?} -> {:?}",
                pool_type, api_type
            );
            if let Some(cred) = self.find_by_api_type(db, pool_type, &api_type)? {
                eprintln!(
                    "[get_fallback_credential] 通过类型映射找到凭证: {:?}",
                    cred.name
                );
                return Ok(Some(cred));
            }
        }

        eprintln!(
            "[get_fallback_credential] 未找到 {:?} 的降级凭证 (provider_id_hint: {:?})",
            pool_type, provider_id_hint
        );
        Ok(None)
    }

    /// PoolProviderType → ApiProviderType 映射
    ///
    /// 仅映射有明确对应关系的类型
    fn map_pool_type_to_api_type(&self, pool_type: &PoolProviderType) -> Option<ApiProviderType> {
        match pool_type {
            // API Key 类型 - 直接映射
            PoolProviderType::Claude => Some(ApiProviderType::Anthropic),
            PoolProviderType::OpenAI => Some(ApiProviderType::Openai),
            PoolProviderType::GeminiApiKey => Some(ApiProviderType::Gemini),
            PoolProviderType::Vertex => Some(ApiProviderType::Vertexai),

            // OAuth 类型 - 可降级到 API Key
            PoolProviderType::Gemini => Some(ApiProviderType::Gemini), // Gemini OAuth → Gemini API Key

            // API Key Provider 类型 - 直接映射
            PoolProviderType::Anthropic => Some(ApiProviderType::Anthropic),
            PoolProviderType::AnthropicCompatible => Some(ApiProviderType::AnthropicCompatible),
            PoolProviderType::AzureOpenai => Some(ApiProviderType::AzureOpenai),
            PoolProviderType::AwsBedrock => Some(ApiProviderType::AwsBedrock),
            PoolProviderType::Ollama => Some(ApiProviderType::Ollama),

            // OAuth-only，无降级
            PoolProviderType::Kiro => None,
            PoolProviderType::Codex => None,
            PoolProviderType::ClaudeOAuth => None,
            PoolProviderType::Antigravity => None,
        }
    }

    /// 通过 ApiProviderType 查找凭证
    fn find_by_api_type(
        &self,
        db: &DbConnection,
        pool_type: &PoolProviderType,
        api_type: &ApiProviderType,
    ) -> Result<Option<ProviderCredential>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // 查找该类型的启用的 Provider（按 sort_order 排序）
        let providers = ApiKeyProviderDao::get_all_providers(&conn).map_err(|e| e.to_string())?;

        let matching_providers: Vec<_> = providers
            .into_iter()
            .filter(|p| p.enabled && p.provider_type == *api_type)
            .collect();

        if matching_providers.is_empty() {
            return Ok(None);
        }

        // 尝试从每个匹配的 Provider 获取可用的 API Key
        for provider in matching_providers {
            let keys = ApiKeyProviderDao::get_enabled_api_keys_by_provider(&conn, &provider.id)
                .map_err(|e| e.to_string())?;

            if keys.is_empty() {
                continue;
            }

            // 轮询选择 API Key
            let index = {
                let mut indices = self.round_robin_index.write().map_err(|e| e.to_string())?;
                indices
                    .entry(provider.id.clone())
                    .or_insert_with(|| AtomicUsize::new(0))
                    .fetch_add(1, Ordering::SeqCst)
            };

            let selected_key = &keys[index % keys.len()];

            // 解密 API Key
            let api_key = self.encryption.decrypt(&selected_key.api_key_encrypted)?;

            // 转换为 ProviderCredential
            let credential = self.convert_to_provider_credential(
                pool_type,
                api_type,
                &provider,
                &selected_key.id,
                &api_key,
            )?;

            tracing::info!(
                "[智能降级] 成功找到凭证: {:?} -> {} (key: {})",
                pool_type,
                provider.name,
                selected_key.alias.as_deref().unwrap_or(&selected_key.id)
            );

            return Ok(Some(credential));
        }

        Ok(None)
    }

    /// 通过 provider_id 直接查找凭证 (支持 60+ Provider)
    ///
    /// 例如: "deepseek", "dashscope", "openrouter"
    async fn find_by_provider_id(
        &self,
        db: &DbConnection,
        provider_id: &str,
        client_type: Option<&crate::server::client_detector::ClientType>,
    ) -> Result<Option<ProviderCredential>, String> {
        // First, get all data we need while holding the lock
        let (provider, keys) = {
            let conn = db.lock().map_err(|e| e.to_string())?;

            // 直接按 provider_id 查找
            let provider = ApiKeyProviderDao::get_provider_by_id(&conn, provider_id)
                .map_err(|e| e.to_string())?;

            let provider = match provider {
                Some(p) if p.enabled => {
                    eprintln!(
                        "[find_by_provider_id] 找到已启用的 provider: id={}, name={}, api_host={}, type={:?}",
                        p.id, p.name, p.api_host, p.provider_type
                    );
                    p
                }
                Some(_p) => {
                    eprintln!(
                        "[find_by_provider_id] provider '{}' 存在但未启用",
                        provider_id
                    );
                    return Ok(None);
                }
                None => {
                    eprintln!("[find_by_provider_id] provider '{}' 不存在", provider_id);
                    return Ok(None);
                }
            };

            // 获取启用的 API Key
            let keys = ApiKeyProviderDao::get_enabled_api_keys_by_provider(&conn, &provider.id)
                .map_err(|e| e.to_string())?;

            if keys.is_empty() {
                eprintln!(
                    "[find_by_provider_id] provider '{}' 没有启用的 API Key",
                    provider_id
                );
                return Ok(None);
            }

            eprintln!(
                "[find_by_provider_id] provider '{}' 有 {} 个启用的 API Key",
                provider_id,
                keys.len()
            );

            (provider, keys)
        }; // conn is released here

        // 轮询选择 API Key，但需要检查客户端兼容性
        let mut selected_key = None;
        let mut attempts = 0;
        let max_attempts = keys.len();

        while attempts < max_attempts {
            let index = {
                let mut indices = self.round_robin_index.write().map_err(|e| e.to_string())?;
                indices
                    .entry(provider.id.clone())
                    .or_insert_with(|| AtomicUsize::new(0))
                    .fetch_add(1, Ordering::SeqCst)
            };

            let candidate_key = &keys[index % keys.len()];

            // 解密 API Key 进行测试
            let api_key = self.encryption.decrypt(&candidate_key.api_key_encrypted)?;

            // 检查客户端兼容性（仅对 Anthropic 类型进行检查）
            if provider.provider_type == ApiProviderType::Anthropic {
                if let Some(client) = client_type {
                    // 对于 Claude Code 客户端，可以使用任何 Claude 凭证
                    if matches!(
                        client,
                        crate::server::client_detector::ClientType::ClaudeCode
                    ) {
                        selected_key = Some(candidate_key);
                        break;
                    }

                    // 对于其他客户端，需要检查凭证是否是 Claude Code 专用
                    // 通过发送测试请求来检查
                    if let Err(e) = self
                        .test_claude_key_compatibility(&api_key, &provider.api_host)
                        .await
                    {
                        if e.contains("CLAUDE_CODE_ONLY") {
                            eprintln!(
                                "[find_by_provider_id] API Key {} 是 Claude Code 专用，跳过 (客户端: {:?})",
                                candidate_key.alias.as_deref().unwrap_or(&candidate_key.id),
                                client
                            );
                            attempts += 1;
                            continue;
                        }
                    }
                }
            }

            selected_key = Some(candidate_key);
            break;
        }

        let selected_key = match selected_key {
            Some(key) => key,
            None => {
                eprintln!(
                    "[find_by_provider_id] provider '{}' 的所有 API Key 都不兼容当前客户端 ({:?})",
                    provider_id, client_type
                );
                return Ok(None);
            }
        };

        // 解密 API Key
        let api_key = self.encryption.decrypt(&selected_key.api_key_encrypted)?;

        // 根据 Provider 类型转换为对应的 ProviderCredential
        let credential =
            self.convert_provider_to_credential(&provider, &selected_key.id, &api_key)?;

        tracing::info!(
            "[智能降级] 成功通过 provider_id 找到凭证: {} (key: {}, type: {:?})",
            provider.name,
            selected_key.alias.as_deref().unwrap_or(&selected_key.id),
            provider.provider_type
        );

        Ok(Some(credential))
    }

    /// 根据 Provider 类型转换为对应的 ProviderCredential
    fn convert_provider_to_credential(
        &self,
        provider: &ApiKeyProvider,
        key_id: &str,
        api_key: &str,
    ) -> Result<ProviderCredential, String> {
        let (credential_data, pool_type) = match provider.provider_type {
            ApiProviderType::Anthropic => {
                // Anthropic 类型使用 ClaudeKey
                let data = CredentialData::ClaudeKey {
                    api_key: api_key.to_string(),
                    base_url: Some(provider.api_host.clone()),
                };
                (data, PoolProviderType::Claude)
            }
            ApiProviderType::AnthropicCompatible => {
                // Anthropic 兼容格式使用 ClaudeKey（与 Anthropic 相同的凭证数据）
                // 但使用 AnthropicCompatible 作为 PoolProviderType，以便使用正确的端点
                let data = CredentialData::ClaudeKey {
                    api_key: api_key.to_string(),
                    base_url: Some(provider.api_host.clone()),
                };
                (data, PoolProviderType::AnthropicCompatible)
            }
            ApiProviderType::Gemini => {
                // Gemini 类型使用 GeminiApiKey
                let data = CredentialData::GeminiApiKey {
                    api_key: api_key.to_string(),
                    base_url: Some(provider.api_host.clone()),
                    excluded_models: Vec::new(),
                };
                (data, PoolProviderType::GeminiApiKey)
            }
            _ => {
                // 其他类型（OpenAI 兼容）使用 OpenAIKey
                let data = CredentialData::OpenAIKey {
                    api_key: api_key.to_string(),
                    base_url: Some(provider.api_host.clone()),
                };
                (data, PoolProviderType::OpenAI)
            }
        };

        let now = chrono::Utc::now();
        Ok(ProviderCredential {
            uuid: format!("fallback-{}", key_id),
            provider_type: pool_type,
            credential: credential_data,
            name: Some(format!("[降级] {}", provider.name)),
            is_healthy: true,
            is_disabled: false,
            check_health: false,
            check_model_name: None,
            not_supported_models: Vec::new(),
            supported_models: Vec::new(),
            usage_count: 0,
            error_count: 0,
            last_used: None,
            last_error_time: None,
            last_error_message: None,
            last_health_check_time: None,
            last_health_check_model: None,
            created_at: now,
            updated_at: now,
            cached_token: None,
            source: CredentialSource::Imported,
            proxy_url: None,
        })
    }

    /// 转换为 ProviderCredential
    fn convert_to_provider_credential(
        &self,
        pool_type: &PoolProviderType,
        api_type: &ApiProviderType,
        provider: &ApiKeyProvider,
        key_id: &str,
        api_key: &str,
    ) -> Result<ProviderCredential, String> {
        let credential_data = match api_type {
            ApiProviderType::Anthropic => CredentialData::ClaudeKey {
                api_key: api_key.to_string(),
                base_url: Some(provider.api_host.clone()),
            },
            ApiProviderType::Gemini => CredentialData::GeminiApiKey {
                api_key: api_key.to_string(),
                base_url: Some(provider.api_host.clone()),
                excluded_models: Vec::new(),
            },
            ApiProviderType::Vertexai => CredentialData::VertexKey {
                api_key: api_key.to_string(),
                base_url: Some(provider.api_host.clone()),
                model_aliases: std::collections::HashMap::new(),
            },
            // 其他类型（包括 Openai, OpenaiResponse 等）都用 OpenAI Key 格式
            _ => CredentialData::OpenAIKey {
                api_key: api_key.to_string(),
                base_url: Some(provider.api_host.clone()),
            },
        };

        let now = chrono::Utc::now();
        Ok(ProviderCredential {
            uuid: format!("fallback-{}", key_id),
            provider_type: *pool_type,
            credential: credential_data,
            name: Some(format!("[降级] {}", provider.name)),
            is_healthy: true,
            is_disabled: false,
            check_health: false, // 降级凭证不参与健康检查
            check_model_name: None,
            not_supported_models: Vec::new(),
            supported_models: Vec::new(),
            usage_count: 0,
            error_count: 0,
            last_used: None,
            last_error_time: None,
            last_error_message: None,
            last_health_check_time: None,
            last_health_check_model: None,
            created_at: now,
            updated_at: now,
            cached_token: None,
            source: CredentialSource::Imported, // 标记为导入来源
            proxy_url: None,
        })
    }

    // ==================== 连接测试 ====================

    /// 测试 Provider 连接
    ///
    /// 方案 C 实现：
    /// 1. 默认使用 /v1/models 端点测试
    /// 2. 如果 Provider 配置了自定义模型列表，用第一个模型发送简单请求
    ///
    /// # 参数
    /// - `db`: 数据库连接
    /// - `provider_id`: Provider ID
    /// - `model_name`: 可选的模型名称，用于发送测试请求
    ///
    /// # 返回
    /// - `ConnectionTestResult`: 测试结果
    pub async fn test_connection(
        &self,
        db: &DbConnection,
        provider_id: &str,
        model_name: Option<String>,
    ) -> Result<ConnectionTestResult, String> {
        use std::time::Instant;

        // 获取 Provider 信息
        let provider_with_keys = self
            .get_provider(db, provider_id)?
            .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

        let provider = &provider_with_keys.provider;

        // 获取一个可用的 API Key
        let api_key = self
            .get_next_api_key(db, provider_id)?
            .ok_or_else(|| "没有可用的 API Key".to_string())?;

        let start_time = Instant::now();

        // 根据 Provider 类型选择测试方式
        let result = match provider.provider_type {
            ApiProviderType::Anthropic => {
                // Anthropic 不支持 /models 端点，需要发送测试请求
                let test_model = model_name
                    .or_else(|| provider.custom_models.first().cloned())
                    .unwrap_or_else(|| "claude-3-haiku-20240307".to_string());

                match self
                    .test_anthropic_connection(&api_key, &provider.api_host, &test_model)
                    .await
                {
                    Ok(models) => Ok(models),
                    Err(e) if e == "CLAUDE_CODE_ONLY" => {
                        // Claude Code 专用凭证限制错误，返回特殊错误信息
                        Err(
                            "凭证限制: 当前 Claude 凭证只能用于 Claude Code，不能用于通用 API 调用"
                                .to_string(),
                        )
                    }
                    Err(e) => Err(e),
                }
            }
            ApiProviderType::Gemini => {
                // Gemini 使用 /models 端点
                self.test_gemini_connection(&api_key, &provider.api_host)
                    .await
            }
            _ => {
                // OpenAI 兼容类型，优先使用 /models 端点
                let models_result = self
                    .test_openai_models_endpoint(&api_key, &provider.api_host)
                    .await;

                // 如果 /models 端点失败且有模型名称，尝试发送测试请求
                if models_result.is_err() && model_name.is_some() {
                    let test_model = model_name.unwrap();
                    self.test_openai_chat_completion(&api_key, &provider.api_host, &test_model)
                        .await
                } else {
                    models_result
                }
            }
        };

        let latency_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(models) => Ok(ConnectionTestResult {
                success: true,
                latency_ms: Some(latency_ms),
                error: None,
                models: Some(models),
            }),
            Err(e) => Ok(ConnectionTestResult {
                success: false,
                latency_ms: Some(latency_ms),
                error: Some(e),
                models: None,
            }),
        }
    }

    /// 测试 OpenAI 兼容的 /models 端点
    async fn test_openai_models_endpoint(
        &self,
        api_key: &str,
        api_host: &str,
    ) -> Result<Vec<String>, String> {
        use crate::providers::openai_custom::OpenAICustomProvider;

        let provider =
            OpenAICustomProvider::with_config(api_key.to_string(), Some(api_host.to_string()));

        let response = provider
            .list_models()
            .await
            .map_err(|e| format!("获取模型列表失败: {}", e))?;

        // 解析模型列表
        let models: Vec<String> = response["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if models.is_empty() {
            Err("未获取到任何模型".to_string())
        } else {
            Ok(models)
        }
    }

    /// 测试 OpenAI 兼容的 chat/completions 端点
    async fn test_openai_chat_completion(
        &self,
        api_key: &str,
        api_host: &str,
        model: &str,
    ) -> Result<Vec<String>, String> {
        use crate::models::openai::{ChatCompletionRequest, ChatMessage, MessageContent};
        use crate::providers::openai_custom::OpenAICustomProvider;

        let provider =
            OpenAICustomProvider::with_config(api_key.to_string(), Some(api_host.to_string()));

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(MessageContent::Text("hi".to_string())),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            temperature: None,
            max_tokens: Some(1),
            top_p: None,
            stream: false,
            tools: None,
            tool_choice: None,
            reasoning_effort: None,
        };

        let response = provider
            .call_api(&request)
            .await
            .map_err(|e| format!("API 调用失败: {}", e))?;

        if response.status().is_success() {
            Ok(vec![model.to_string()])
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(format!("API 返回错误: {} - {}", status, body))
        }
    }

    /// 测试 Claude Key 的客户端兼容性
    async fn test_claude_key_compatibility(
        &self,
        api_key: &str,
        api_host: &str,
    ) -> Result<(), String> {
        use crate::providers::claude_custom::ClaudeCustomProvider;

        let provider =
            ClaudeCustomProvider::with_config(api_key.to_string(), Some(api_host.to_string()));

        // 发送一个最小的测试请求
        let request = serde_json::json!({
            "model": "claude-3-haiku-20240307",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

        let response = provider
            .messages(&request)
            .await
            .map_err(|e| format!("API 调用失败: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();

            // 检查是否是 Claude Code 专用凭证限制错误
            if body.contains("only authorized for use with Claude Code") {
                return Err("CLAUDE_CODE_ONLY".to_string());
            }

            // 其他错误不影响兼容性判断
            Ok(())
        }
    }

    /// 测试 Anthropic 连接
    async fn test_anthropic_connection(
        &self,
        api_key: &str,
        api_host: &str,
        model: &str,
    ) -> Result<Vec<String>, String> {
        use crate::providers::claude_custom::ClaudeCustomProvider;

        let provider =
            ClaudeCustomProvider::with_config(api_key.to_string(), Some(api_host.to_string()));

        // 发送一个简单的测试请求
        let request = serde_json::json!({
            "model": model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

        let response = provider
            .messages(&request)
            .await
            .map_err(|e| format!("API 调用失败: {}", e))?;

        if response.status().is_success() {
            Ok(vec![model.to_string()])
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // 检查是否是 Claude Code 专用凭证限制错误
            if body.contains("only authorized for use with Claude Code") {
                return Err("CLAUDE_CODE_ONLY".to_string());
            }

            Err(format!("API 返回错误: {} - {}", status, body))
        }
    }

    /// 测试 Gemini 连接
    async fn test_gemini_connection(
        &self,
        api_key: &str,
        api_host: &str,
    ) -> Result<Vec<String>, String> {
        use reqwest::Client;
        use std::time::Duration;

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        // Gemini API 的模型列表端点
        let base = api_host.trim_end_matches('/');
        let url = format!("{}/v1beta/models?key={}", base, api_key);

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API 返回错误: {} - {}", status, body));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let models: Vec<String> = data["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if models.is_empty() {
            Err("未获取到任何模型".to_string())
        } else {
            Ok(models)
        }
    }
}

/// 导入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub imported_providers: usize,
    pub imported_api_keys: usize,
    pub skipped_providers: usize,
    pub errors: Vec<String>,
}
