//! 模型注册服务
//!
//! 从内嵌资源加载模型数据，管理本地缓存，提供模型搜索等功能
//! 模型数据在构建时从 aiclientproxy/models 仓库打包进应用

use proxycast_core::database::dao::api_key_provider::ApiProviderType;
use proxycast_core::database::DbConnection;
use proxycast_core::models::model_registry::{
    EnhancedModelMetadata, ModelCapabilities, ModelLimits, ModelPricing, ModelSource, ModelStatus,
    ModelSyncState, ModelTier, ProviderAliasConfig, UserModelPreference,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 内嵌的模型资源目录名（相对于 resource_dir）
/// 对应 tauri.conf.json 中的 "resources/models/**/*"
const MODELS_RESOURCE_DIR: &str = "resources/models";

/// 仓库索引文件结构
#[derive(Debug, Deserialize)]
struct RepoIndex {
    providers: Vec<String>,
    #[allow(dead_code)]
    total_models: u32,
}

/// 仓库中的 Provider 数据结构
#[derive(Debug, Deserialize)]
struct RepoProviderData {
    provider: RepoProvider,
    models: Vec<RepoModel>,
}

#[derive(Debug, Deserialize)]
struct RepoProvider {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct RepoModel {
    id: String,
    name: String,
    family: Option<String>,
    tier: Option<String>,
    capabilities: Option<RepoCapabilities>,
    pricing: Option<RepoPricing>,
    limits: Option<RepoLimits>,
    status: Option<String>,
    release_date: Option<String>,
    is_latest: Option<bool>,
    description: Option<String>,
    #[serde(default)]
    description_zh: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RepoCapabilities {
    #[serde(default)]
    vision: bool,
    #[serde(default)]
    tools: bool,
    #[serde(default)]
    streaming: bool,
    #[serde(default)]
    json_mode: bool,
    #[serde(default)]
    function_calling: bool,
    #[serde(default)]
    reasoning: bool,
}

#[derive(Debug, Deserialize)]
struct RepoPricing {
    input: Option<f64>,
    output: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
    currency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoLimits {
    context: Option<u32>,
    max_output: Option<u32>,
}

/// 模型注册服务
pub struct ModelRegistryService {
    /// 数据库连接
    db: DbConnection,
    /// 内存缓存的模型数据
    models_cache: Arc<RwLock<Vec<EnhancedModelMetadata>>>,
    /// Provider 别名配置缓存（provider_id -> ProviderAliasConfig）
    aliases_cache: Arc<RwLock<HashMap<String, ProviderAliasConfig>>>,
    /// 同步状态
    sync_state: Arc<RwLock<ModelSyncState>>,
    /// 资源目录路径
    resource_dir: Option<std::path::PathBuf>,
}

impl ModelRegistryService {
    /// 创建新的模型注册服务
    pub fn new(db: DbConnection) -> Self {
        Self {
            db,
            models_cache: Arc::new(RwLock::new(Vec::new())),
            aliases_cache: Arc::new(RwLock::new(HashMap::new())),
            sync_state: Arc::new(RwLock::new(ModelSyncState::default())),
            resource_dir: None,
        }
    }

    /// 设置资源目录路径
    pub fn set_resource_dir(&mut self, path: std::path::PathBuf) {
        self.resource_dir = Some(path);
    }

    /// 初始化服务 - 从内嵌资源加载模型数据
    pub async fn initialize(&self) -> Result<(), String> {
        tracing::info!("[ModelRegistry] 初始化模型注册服务");

        // 始终从内嵌资源加载，不再回退到数据库
        let (models, aliases) = self.load_from_embedded_resources().await?;

        tracing::info!(
            "[ModelRegistry] 从内嵌资源加载了 {} 个模型, {} 个别名配置",
            models.len(),
            aliases.len()
        );

        // 更新缓存
        {
            let mut cache = self.models_cache.write().await;
            *cache = models.clone();
        }
        {
            let mut cache = self.aliases_cache.write().await;
            *cache = aliases;
        }

        // 更新同步状态
        {
            let mut state = self.sync_state.write().await;
            state.model_count = models.len() as u32;
            state.last_sync_at = Some(chrono::Utc::now().timestamp());
            state.is_syncing = false;
            state.last_error = None;
        }

        // 保存到数据库（仅用于持久化，不影响运行时数据）
        if let Err(e) = self.save_models_to_db(&models).await {
            tracing::warn!("[ModelRegistry] 保存模型到数据库失败: {}", e);
        }

        Ok(())
    }

    /// 从内嵌资源加载模型数据
    async fn load_from_embedded_resources(
        &self,
    ) -> Result<
        (
            Vec<EnhancedModelMetadata>,
            HashMap<String, ProviderAliasConfig>,
        ),
        String,
    > {
        let resource_dir = self
            .resource_dir
            .as_ref()
            .ok_or_else(|| "资源目录未设置".to_string())?;

        tracing::info!("[ModelRegistry] resource_dir: {:?}", resource_dir);

        let models_dir = resource_dir.join(MODELS_RESOURCE_DIR);
        let index_file = models_dir.join("index.json");

        tracing::info!("[ModelRegistry] models_dir: {:?}", models_dir);
        tracing::info!(
            "[ModelRegistry] index_file: {:?}, exists: {}",
            index_file,
            index_file.exists()
        );

        if !index_file.exists() {
            return Err(format!("索引文件不存在: {index_file:?}"));
        }

        // 1. 读取索引文件
        let index_content =
            std::fs::read_to_string(&index_file).map_err(|e| format!("读取索引文件失败: {e}"))?;
        let index: RepoIndex =
            serde_json::from_str(&index_content).map_err(|e| format!("解析索引文件失败: {e}"))?;

        tracing::info!(
            "[ModelRegistry] 索引包含 {} 个 providers",
            index.providers.len()
        );

        // 2. 加载所有 provider 数据
        let mut models = Vec::new();
        let now = chrono::Utc::now().timestamp();
        let providers_dir = models_dir.join("providers");

        tracing::info!("[ModelRegistry] providers_dir: {:?}", providers_dir);

        for provider_id in &index.providers {
            let provider_file = providers_dir.join(format!("{provider_id}.json"));

            if !provider_file.exists() {
                tracing::warn!("[ModelRegistry] Provider 文件不存在: {:?}", provider_file);
                continue;
            }

            match std::fs::read_to_string(&provider_file) {
                Ok(content) => match serde_json::from_str::<RepoProviderData>(&content) {
                    Ok(provider_data) => {
                        tracing::info!(
                            "[ModelRegistry] 加载 Provider: {} ({} 个模型)",
                            provider_id,
                            provider_data.models.len()
                        );
                        for model in provider_data.models {
                            let enhanced = self.convert_repo_model(
                                model,
                                &provider_data.provider.id,
                                &provider_data.provider.name,
                                now,
                            );
                            models.push(enhanced);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[ModelRegistry] 解析 {} 失败: {}", provider_id, e);
                    }
                },
                Err(e) => {
                    tracing::warn!("[ModelRegistry] 读取 {} 失败: {}", provider_id, e);
                }
            }
        }

        // 去重：优先保留 provider_id 为 "anthropic" 的模型
        // 对于相同 ID 的模型，anthropic 官方的优先级最高
        let mut seen_ids: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let original_count = models.len();

        let mut to_keep = vec![true; models.len()];
        for (idx, model) in models.iter().enumerate() {
            if let Some(&existing_idx) = seen_ids.get(&model.id) {
                // 已经有相同 ID 的模型
                let existing_model = &models[existing_idx];

                // 如果当前模型是 anthropic 官方的，替换之前的
                if model.provider_id == "anthropic" && existing_model.provider_id != "anthropic" {
                    to_keep[existing_idx] = false;
                    seen_ids.insert(model.id.clone(), idx);
                } else {
                    // 否则保留第一个
                    to_keep[idx] = false;
                }
            } else {
                seen_ids.insert(model.id.clone(), idx);
            }
        }

        models = models
            .into_iter()
            .enumerate()
            .filter_map(|(idx, model)| if to_keep[idx] { Some(model) } else { None })
            .collect();

        if models.len() < original_count {
            tracing::warn!(
                "[ModelRegistry] 发现 {} 个重复 ID，已去重",
                original_count - models.len()
            );
        }

        // 按 provider_id 和 display_name 排序
        models.sort_by(|a, b| {
            a.provider_id
                .cmp(&b.provider_id)
                .then(a.display_name.cmp(&b.display_name))
        });

        // 3. 加载别名配置
        let mut aliases = HashMap::new();
        let aliases_dir = models_dir.join("aliases");
        let alias_files = ["kiro", "antigravity", "codex", "gemini"];

        for alias_name in alias_files {
            let alias_file = aliases_dir.join(format!("{alias_name}.json"));
            if !alias_file.exists() {
                continue;
            }

            match std::fs::read_to_string(&alias_file) {
                Ok(content) => match serde_json::from_str::<ProviderAliasConfig>(&content) {
                    Ok(config) => {
                        tracing::info!(
                            "[ModelRegistry] 加载别名配置: {} ({} 个模型)",
                            config.provider,
                            config.models.len()
                        );
                        aliases.insert(config.provider.clone(), config);
                    }
                    Err(e) => {
                        tracing::warn!("[ModelRegistry] 解析别名配置 {} 失败: {}", alias_name, e);
                    }
                },
                Err(e) => {
                    tracing::warn!("[ModelRegistry] 读取别名配置 {} 失败: {}", alias_name, e);
                }
            }
        }

        tracing::info!("[ModelRegistry] 从内嵌资源加载了 {} 个模型", models.len());

        Ok((models, aliases))
    }

    /// 转换仓库模型格式为内部格式
    fn convert_repo_model(
        &self,
        model: RepoModel,
        provider_id: &str,
        provider_name: &str,
        now: i64,
    ) -> EnhancedModelMetadata {
        let caps = model.capabilities.unwrap_or_default();

        EnhancedModelMetadata {
            id: model.id,
            display_name: model.name,
            provider_id: provider_id.to_string(),
            provider_name: provider_name.to_string(),
            family: model.family,
            tier: model
                .tier
                .and_then(|t| t.parse().ok())
                .unwrap_or(ModelTier::Pro),
            capabilities: ModelCapabilities {
                vision: caps.vision,
                tools: caps.tools,
                streaming: caps.streaming,
                json_mode: caps.json_mode,
                function_calling: caps.function_calling,
                reasoning: caps.reasoning,
            },
            pricing: model.pricing.map(|p| ModelPricing {
                input_per_million: p.input,
                output_per_million: p.output,
                cache_read_per_million: p.cache_read,
                cache_write_per_million: p.cache_write,
                currency: p.currency.unwrap_or_else(|| "USD".to_string()),
            }),
            limits: ModelLimits {
                context_length: model.limits.as_ref().and_then(|l| l.context),
                max_output_tokens: model.limits.as_ref().and_then(|l| l.max_output),
                requests_per_minute: None,
                tokens_per_minute: None,
            },
            status: model
                .status
                .and_then(|s| s.parse().ok())
                .unwrap_or(ModelStatus::Active),
            release_date: model.release_date,
            is_latest: model.is_latest.unwrap_or(false),
            description: model.description_zh.or(model.description),
            source: ModelSource::Embedded,
            created_at: now,
            updated_at: now,
        }
    }

    /// 从数据库加载模型（预留，将来实现从数据库加载自定义模型）
    #[allow(dead_code)]
    async fn load_from_db(&self) -> Result<Vec<EnhancedModelMetadata>, String> {
        let (models, sync_rows) = {
            let conn = self.db.lock().map_err(|e| e.to_string())?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, display_name, provider_id, provider_name, family, tier,
                            capabilities, pricing, limits, status, release_date, is_latest,
                            description, source, created_at, updated_at
                     FROM model_registry",
                )
                .map_err(|e| e.to_string())?;

            let models = stmt
                .query_map([], |row| {
                    let capabilities_json: String = row.get(6)?;
                    let pricing_json: Option<String> = row.get(7)?;
                    let limits_json: String = row.get(8)?;
                    let status_str: String = row.get(9)?;
                    let tier_str: String = row.get(5)?;
                    let source_str: String = row.get(13)?;

                    Ok(EnhancedModelMetadata {
                        id: row.get(0)?,
                        display_name: row.get(1)?,
                        provider_id: row.get(2)?,
                        provider_name: row.get(3)?,
                        family: row.get(4)?,
                        tier: tier_str.parse().unwrap_or(ModelTier::Pro),
                        capabilities: serde_json::from_str(&capabilities_json).unwrap_or_default(),
                        pricing: pricing_json.and_then(|s| serde_json::from_str(&s).ok()),
                        limits: serde_json::from_str(&limits_json).unwrap_or_default(),
                        status: status_str.parse().unwrap_or(ModelStatus::Active),
                        release_date: row.get(10)?,
                        is_latest: row.get::<_, i32>(11)? != 0,
                        description: row.get(12)?,
                        source: source_str.parse().unwrap_or(ModelSource::Local),
                        created_at: row.get(14)?,
                        updated_at: row.get(15)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            // 加载同步状态数据
            let mut sync_stmt = conn
                .prepare("SELECT key, value FROM model_sync_state")
                .map_err(|e| e.to_string())?;

            let sync_rows: Vec<(String, String)> = sync_stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            (models, sync_rows)
        }; // conn 锁在这里释放

        // 更新同步状态（在锁释放后）
        {
            let mut state = self.sync_state.write().await;
            for (key, value) in sync_rows {
                match key.as_str() {
                    "last_sync_at" => {
                        state.last_sync_at = value.parse().ok();
                    }
                    "model_count" => {
                        state.model_count = value.parse().unwrap_or(0);
                    }
                    "last_error" => {
                        state.last_error = if value.is_empty() { None } else { Some(value) };
                    }
                    _ => {}
                }
            }
        }

        Ok(models)
    }

    /// 保存模型到数据库
    async fn save_models_to_db(&self, models: &[EnhancedModelMetadata]) -> Result<(), String> {
        let mut conn = self.db.lock().map_err(|e| e.to_string())?;

        // 使用 rusqlite 的事务 API
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        // 清空现有数据
        tx.execute("DELETE FROM model_registry", [])
            .map_err(|e| e.to_string())?;

        // 插入新数据（使用 INSERT OR REPLACE 处理可能的重复 ID）
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO model_registry (
                        id, display_name, provider_id, provider_name, family, tier,
                        capabilities, pricing, limits, status, release_date, is_latest,
                        description, source, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .map_err(|e| e.to_string())?;

            for model in models {
                let capabilities_json =
                    serde_json::to_string(&model.capabilities).unwrap_or_default();
                let pricing_json = model
                    .pricing
                    .as_ref()
                    .map(|p| serde_json::to_string(p).unwrap_or_default());
                let limits_json = serde_json::to_string(&model.limits).unwrap_or_default();

                stmt.execute(params![
                    model.id,
                    model.display_name,
                    model.provider_id,
                    model.provider_name,
                    model.family,
                    model.tier.to_string(),
                    capabilities_json,
                    pricing_json,
                    limits_json,
                    model.status.to_string(),
                    model.release_date,
                    model.is_latest as i32,
                    model.description,
                    model.source.to_string(),
                    model.created_at,
                    model.updated_at,
                ])
                .map_err(|e| e.to_string())?;
            }
        }

        // 提交事务
        tx.commit().map_err(|e| e.to_string())?;

        tracing::info!("[ModelRegistry] 保存了 {} 个模型到数据库", models.len());

        Ok(())
    }

    /// 获取所有模型
    pub async fn get_all_models(&self) -> Vec<EnhancedModelMetadata> {
        self.models_cache.read().await.clone()
    }

    /// 获取同步状态
    pub async fn get_sync_state(&self) -> ModelSyncState {
        self.sync_state.read().await.clone()
    }

    /// 强制从内嵌资源重新加载模型数据
    ///
    /// 清除数据库缓存并重新从资源文件加载最新的模型数据
    pub async fn force_reload(&self) -> Result<u32, String> {
        tracing::info!("[ModelRegistry] 强制重新加载模型数据");

        // 从内嵌资源加载
        let (models, aliases) = self.load_from_embedded_resources().await?;

        let model_count = models.len() as u32;
        tracing::info!(
            "[ModelRegistry] 从内嵌资源加载了 {} 个模型, {} 个别名配置",
            models.len(),
            aliases.len()
        );

        // 更新缓存
        {
            let mut cache = self.models_cache.write().await;
            *cache = models.clone();
        }
        {
            let mut cache = self.aliases_cache.write().await;
            *cache = aliases;
        }

        // 更新同步状态
        {
            let mut state = self.sync_state.write().await;
            state.model_count = model_count;
            state.last_sync_at = Some(chrono::Utc::now().timestamp());
            state.is_syncing = false;
            state.last_error = None;
        }

        // 保存到数据库
        self.save_models_to_db(&models).await?;

        Ok(model_count)
    }

    /// 按 Provider 获取模型
    pub async fn get_models_by_provider(&self, provider_id: &str) -> Vec<EnhancedModelMetadata> {
        self.models_cache
            .read()
            .await
            .iter()
            .filter(|m| m.provider_id == provider_id)
            .cloned()
            .collect()
    }

    /// 按服务等级获取模型
    pub async fn get_models_by_tier(&self, tier: ModelTier) -> Vec<EnhancedModelMetadata> {
        self.models_cache
            .read()
            .await
            .iter()
            .filter(|m| m.tier == tier)
            .cloned()
            .collect()
    }

    /// 搜索模型（简单的模糊匹配）
    pub async fn search_models(&self, query: &str, limit: usize) -> Vec<EnhancedModelMetadata> {
        let models = self.models_cache.read().await;

        if query.is_empty() {
            return models.iter().take(limit).cloned().collect();
        }

        let query_lower = query.to_lowercase();
        let mut scored: Vec<(f64, &EnhancedModelMetadata)> = models
            .iter()
            .filter_map(|m| {
                let score = self.calculate_search_score(m, &query_lower);
                if score > 0.0 {
                    Some((score, m))
                } else {
                    None
                }
            })
            .collect();

        // 按分数降序排序
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(limit)
            .map(|(_, m)| m.clone())
            .collect()
    }

    /// 计算搜索匹配分数
    fn calculate_search_score(&self, model: &EnhancedModelMetadata, query: &str) -> f64 {
        let mut score = 0.0;

        // 精确匹配 ID
        if model.id.to_lowercase() == query {
            score += 100.0;
        } else if model.id.to_lowercase().contains(query) {
            score += 50.0;
        }

        // 显示名称匹配
        if model.display_name.to_lowercase().contains(query) {
            score += 30.0;
        }

        // Provider 匹配
        if model.provider_name.to_lowercase().contains(query) {
            score += 20.0;
        }

        // 家族匹配
        if let Some(family) = &model.family {
            if family.to_lowercase().contains(query) {
                score += 15.0;
            }
        }

        // 最新版本加分
        if model.is_latest {
            score += 5.0;
        }

        // 活跃状态加分
        if model.status == ModelStatus::Active {
            score += 3.0;
        }

        score
    }

    // ========== 用户偏好相关方法 ==========

    /// 获取所有用户偏好
    pub async fn get_all_preferences(&self) -> Result<Vec<UserModelPreference>, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT model_id, is_favorite, is_hidden, custom_alias,
                        usage_count, last_used_at, created_at, updated_at
                 FROM user_model_preferences",
            )
            .map_err(|e| e.to_string())?;

        let prefs = stmt
            .query_map([], |row| {
                Ok(UserModelPreference {
                    model_id: row.get(0)?,
                    is_favorite: row.get::<_, i32>(1)? != 0,
                    is_hidden: row.get::<_, i32>(2)? != 0,
                    custom_alias: row.get(3)?,
                    usage_count: row.get::<_, i32>(4)? as u32,
                    last_used_at: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(prefs)
    }

    /// 切换收藏状态
    pub async fn toggle_favorite(&self, model_id: &str) -> Result<bool, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();

        // 检查是否存在
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM user_model_preferences WHERE model_id = ?",
                params![model_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if exists {
            // 切换状态
            conn.execute(
                "UPDATE user_model_preferences
                 SET is_favorite = NOT is_favorite, updated_at = ?
                 WHERE model_id = ?",
                params![now, model_id],
            )
            .map_err(|e| e.to_string())?;
        } else {
            // 创建新记录
            conn.execute(
                "INSERT INTO user_model_preferences
                 (model_id, is_favorite, is_hidden, usage_count, created_at, updated_at)
                 VALUES (?, 1, 0, 0, ?, ?)",
                params![model_id, now, now],
            )
            .map_err(|e| e.to_string())?;
        }

        // 返回新状态
        let new_state: bool = conn
            .query_row(
                "SELECT is_favorite FROM user_model_preferences WHERE model_id = ?",
                params![model_id],
                |row| Ok(row.get::<_, i32>(0)? != 0),
            )
            .unwrap_or(false);

        Ok(new_state)
    }

    /// 隐藏模型
    pub async fn hide_model(&self, model_id: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO user_model_preferences
             (model_id, is_favorite, is_hidden, usage_count, created_at, updated_at)
             VALUES (?, 0, 1, 0, ?, ?)
             ON CONFLICT(model_id) DO UPDATE SET is_hidden = 1, updated_at = ?",
            params![model_id, now, now, now],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 记录模型使用
    pub async fn record_usage(&self, model_id: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO user_model_preferences
             (model_id, is_favorite, is_hidden, usage_count, last_used_at, created_at, updated_at)
             VALUES (?, 0, 0, 1, ?, ?, ?)
             ON CONFLICT(model_id) DO UPDATE SET
                usage_count = usage_count + 1,
                last_used_at = ?,
                updated_at = ?",
            params![model_id, now, now, now, now, now],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    // ========== Provider 别名相关方法 ==========

    /// 获取指定 Provider 的别名配置
    pub async fn get_provider_alias_config(&self, provider: &str) -> Option<ProviderAliasConfig> {
        self.aliases_cache.read().await.get(provider).cloned()
    }

    /// 检查指定 Provider 是否支持某个模型
    pub async fn provider_supports_model(&self, provider: &str, model: &str) -> bool {
        if let Some(config) = self.aliases_cache.read().await.get(provider) {
            config.supports_model(model)
        } else {
            // 如果没有别名配置，默认支持所有模型
            true
        }
    }

    /// 获取模型在指定 Provider 中的内部名称
    pub async fn get_model_internal_name(&self, provider: &str, model: &str) -> Option<String> {
        self.aliases_cache
            .read()
            .await
            .get(provider)
            .and_then(|config| config.get_internal_name(model).map(|s| s.to_string()))
    }

    /// 获取所有 Provider 别名配置
    pub async fn get_all_alias_configs(&self) -> HashMap<String, ProviderAliasConfig> {
        self.aliases_cache.read().await.clone()
    }

    // ========== 从 Provider API 获取模型 ==========

    /// 从 Provider API 获取模型列表
    ///
    /// 调用 Provider 的 /v1/models 端点获取模型列表，
    /// 如果失败则回退到本地 JSON 文件
    ///
    /// # 参数
    /// - `provider_id`: Provider ID（如 "siliconflow", "openai"）
    /// - `api_host`: API 主机地址
    /// - `api_key`: API Key
    ///
    /// # 返回
    /// - `Ok(FetchModelsResult)`: 获取结果，包含模型列表和来源
    pub async fn fetch_models_from_api(
        &self,
        provider_id: &str,
        api_host: &str,
        api_key: &str,
    ) -> Result<FetchModelsResult, String> {
        self.fetch_models_from_api_with_hints(provider_id, api_host, api_key, None, &[])
            .await
    }

    /// 从 Provider API 获取模型列表（带兜底提示）
    ///
    /// 优先使用 API 实时结果；当 API 不可用时，按以下顺序进行本地兜底：
    /// 1. 精确匹配 custom_models
    /// 2. 按 provider_id / provider_type / api_host 推断候选 provider
    /// 3. 使用本地资源中的候选 provider 模型列表
    pub async fn fetch_models_from_api_with_hints(
        &self,
        provider_id: &str,
        api_host: &str,
        api_key: &str,
        provider_type: Option<ApiProviderType>,
        custom_models: &[String],
    ) -> Result<FetchModelsResult, String> {
        tracing::info!(
            "[ModelRegistry] 从 API 获取模型: provider={}, host={}",
            provider_id,
            api_host
        );

        // 构建 API URL
        let api_url = Self::build_models_api_url(api_host);
        tracing::info!("[ModelRegistry] API URL: {}", api_url);

        // 尝试从 API 获取
        match self.call_models_api(&api_url, api_key).await {
            Ok(api_models) => {
                tracing::info!("[ModelRegistry] 从 API 获取到 {} 个模型", api_models.len());

                // 转换为内部格式
                let now = chrono::Utc::now().timestamp();
                let models: Vec<EnhancedModelMetadata> = api_models
                    .into_iter()
                    .map(|m| self.convert_api_model(m, provider_id, now))
                    .collect();

                Ok(FetchModelsResult {
                    models,
                    source: ModelFetchSource::Api,
                    error: None,
                })
            }
            Err(api_error) => {
                tracing::warn!(
                    "[ModelRegistry] API 获取失败: {}, 回退到本地文件",
                    api_error
                );

                // 回退到本地资源模型（多级匹配）
                let local_models = self
                    .resolve_local_fallback_models(
                        provider_id,
                        api_host,
                        provider_type,
                        custom_models,
                    )
                    .await;

                if local_models.is_empty() {
                    Ok(FetchModelsResult {
                        models: vec![],
                        source: ModelFetchSource::LocalFallback,
                        error: Some(format!("API 获取失败: {api_error}, 本地也无数据")),
                    })
                } else {
                    Ok(FetchModelsResult {
                        models: local_models,
                        source: ModelFetchSource::LocalFallback,
                        error: Some(format!("API 获取失败: {api_error}, 已使用本地数据")),
                    })
                }
            }
        }
    }

    async fn resolve_local_fallback_models(
        &self,
        provider_id: &str,
        api_host: &str,
        provider_type: Option<ApiProviderType>,
        custom_models: &[String],
    ) -> Vec<EnhancedModelMetadata> {
        // 优先精确匹配 custom_models（最贴近用户配置）
        let matched_custom_models = self.match_local_models_by_ids(custom_models).await;
        if !matched_custom_models.is_empty() {
            tracing::info!(
                "[ModelRegistry] 本地兜底命中 custom_models: provider={}, matched={}",
                provider_id,
                matched_custom_models.len()
            );
            return matched_custom_models;
        }

        let candidate_provider_ids = self
            .collect_fallback_provider_ids(provider_id, api_host, provider_type, custom_models)
            .await;

        tracing::info!(
            "[ModelRegistry] 本地兜底候选 provider: provider={}, candidates={:?}",
            provider_id,
            candidate_provider_ids
        );

        let cache = self.models_cache.read().await;
        let mut models = Vec::new();
        let mut seen_ids = HashSet::new();

        for candidate in &candidate_provider_ids {
            for model in cache.iter().filter(|m| m.provider_id == *candidate) {
                if seen_ids.insert(model.id.clone()) {
                    models.push(model.clone());
                }
            }
        }

        models
    }

    async fn match_local_models_by_ids(&self, model_ids: &[String]) -> Vec<EnhancedModelMetadata> {
        if model_ids.is_empty() {
            return Vec::new();
        }

        let target_ids: HashSet<String> = model_ids
            .iter()
            .map(|id| id.trim().to_lowercase())
            .filter(|id| !id.is_empty())
            .collect();

        if target_ids.is_empty() {
            return Vec::new();
        }

        let cache = self.models_cache.read().await;
        cache
            .iter()
            .filter(|m| target_ids.contains(&m.id.to_lowercase()))
            .cloned()
            .collect()
    }

    async fn collect_fallback_provider_ids(
        &self,
        provider_id: &str,
        api_host: &str,
        provider_type: Option<ApiProviderType>,
        custom_models: &[String],
    ) -> Vec<String> {
        let mut candidates = Vec::new();

        Self::push_unique_candidate(&mut candidates, provider_id);

        if let Some(stripped) = provider_id.strip_suffix("_api_key") {
            Self::push_unique_candidate(&mut candidates, stripped);
        }

        // 根据 custom_models 反推 provider（可覆盖 custom-* 场景）
        if !custom_models.is_empty() {
            let target_ids: HashSet<String> = custom_models
                .iter()
                .map(|id| id.trim().to_lowercase())
                .filter(|id| !id.is_empty())
                .collect();

            if !target_ids.is_empty() {
                let cache = self.models_cache.read().await;
                for model in cache.iter() {
                    if target_ids.contains(&model.id.to_lowercase()) {
                        Self::push_unique_candidate(&mut candidates, &model.provider_id);
                    }
                }
            }
        }

        for inferred_id in Self::infer_provider_ids_from_api_host(api_host) {
            Self::push_unique_candidate(&mut candidates, inferred_id);
        }

        if let Some(provider_type) = provider_type {
            for mapped_id in Self::map_provider_type_to_registry_ids(provider_type) {
                Self::push_unique_candidate(&mut candidates, mapped_id);
            }
        }

        candidates
    }

    fn push_unique_candidate(candidates: &mut Vec<String>, candidate: &str) {
        if candidate.trim().is_empty() {
            return;
        }

        let normalized = candidate.trim().to_lowercase();
        if !candidates.iter().any(|existing| existing == &normalized) {
            candidates.push(normalized);
        }
    }

    fn infer_provider_ids_from_api_host(api_host: &str) -> &'static [&'static str] {
        let host = api_host.to_lowercase();

        if host.contains("bigmodel.cn") {
            return &["zhipuai"];
        }

        if host.contains("z.ai") || host.contains("zai") {
            return &["zai"];
        }

        if host.contains("anthropic.com") {
            return &["anthropic"];
        }

        if host.contains("openai.com") {
            return &["openai"];
        }

        if host.contains("googleapis.com") {
            return &["google"];
        }

        if host.contains("bedrock") {
            return &["amazon-bedrock"];
        }

        if host.contains("ollama") {
            return &["ollama-cloud"];
        }

        &[]
    }

    fn map_provider_type_to_registry_ids(
        provider_type: ApiProviderType,
    ) -> &'static [&'static str] {
        match provider_type {
            ApiProviderType::Openai
            | ApiProviderType::OpenaiResponse
            | ApiProviderType::NewApi
            | ApiProviderType::Gateway
            | ApiProviderType::AzureOpenai => &["openai"],
            ApiProviderType::Anthropic | ApiProviderType::AnthropicCompatible => &["anthropic"],
            ApiProviderType::Gemini => &["google"],
            ApiProviderType::Vertexai => &["google-vertex", "google"],
            ApiProviderType::AwsBedrock => &["amazon-bedrock"],
            ApiProviderType::Ollama => &["ollama-cloud"],
            ApiProviderType::Codex => &["codex"],
        }
    }

    /// 构建 /v1/models API URL
    fn build_models_api_url(api_host: &str) -> String {
        let host = api_host.trim_end_matches('/');

        // 检查是否已经包含 /v1 路径
        if host.ends_with("/v1") || host.ends_with("/v1/") {
            format!("{}/models", host.trim_end_matches('/'))
        } else if host.contains("/v1/") {
            // 如果路径中间有 /v1/，直接追加 models
            format!("{}models", host.trim_end_matches('/').to_string() + "/")
        } else {
            format!("{host}/v1/models")
        }
    }

    /// 调用 /v1/models API
    async fn call_models_api(
        &self,
        url: &str,
        api_key: &str,
    ) -> Result<Vec<ApiModelResponse>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| format!("请求失败: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "无法读取响应体".to_string());
            return Err(format!("API 返回错误 {status}: {body}"));
        }

        let body = response
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {e}"))?;

        // 解析 OpenAI 格式的响应
        let api_response: ApiModelsResponse =
            serde_json::from_str(&body).map_err(|e| format!("解析响应失败: {e}"))?;

        Ok(api_response.data)
    }

    /// 转换 API 模型格式为内部格式
    fn convert_api_model(
        &self,
        model: ApiModelResponse,
        provider_id: &str,
        now: i64,
    ) -> EnhancedModelMetadata {
        // 从 model id 推断显示名称
        let display_name = model
            .id
            .split('/')
            .next_back()
            .unwrap_or(&model.id)
            .to_string();

        EnhancedModelMetadata {
            id: model.id.clone(),
            display_name,
            provider_id: provider_id.to_string(),
            provider_name: model.owned_by.unwrap_or_else(|| provider_id.to_string()),
            family: None,
            tier: ModelTier::Pro,
            capabilities: ModelCapabilities {
                vision: false,
                tools: false,
                streaming: true,
                json_mode: false,
                function_calling: false,
                reasoning: false,
            },
            pricing: None,
            limits: ModelLimits {
                context_length: model.context_length,
                max_output_tokens: None,
                requests_per_minute: None,
                tokens_per_minute: None,
            },
            status: ModelStatus::Active,
            release_date: None,
            is_latest: false,
            description: None,
            source: ModelSource::Api,
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================================
// API 响应类型
// ============================================================================

/// OpenAI /v1/models API 响应格式
#[derive(Debug, Deserialize)]
struct ApiModelsResponse {
    data: Vec<ApiModelResponse>,
}

/// 单个模型的 API 响应
#[derive(Debug, Deserialize)]
struct ApiModelResponse {
    id: String,
    #[serde(default)]
    owned_by: Option<String>,
    #[serde(default)]
    context_length: Option<u32>,
}

/// 模型获取来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelFetchSource {
    /// 从 API 获取
    Api,
    /// 从本地文件回退
    LocalFallback,
}

/// 从 API 获取模型的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchModelsResult {
    /// 模型列表
    pub models: Vec<EnhancedModelMetadata>,
    /// 数据来源
    pub source: ModelFetchSource,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::ModelRegistryService;
    use proxycast_core::database::dao::api_key_provider::ApiProviderType;

    #[test]
    fn test_build_models_api_url() {
        assert_eq!(
            ModelRegistryService::build_models_api_url("https://api.openai.com"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            ModelRegistryService::build_models_api_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            ModelRegistryService::build_models_api_url("https://open.bigmodel.cn/api/anthropic"),
            "https://open.bigmodel.cn/api/anthropic/v1/models"
        );
    }

    #[test]
    fn test_infer_provider_ids_from_api_host() {
        assert_eq!(
            ModelRegistryService::infer_provider_ids_from_api_host(
                "https://open.bigmodel.cn/api/anthropic"
            ),
            ["zhipuai"]
        );
        assert_eq!(
            ModelRegistryService::infer_provider_ids_from_api_host("https://api.openai.com/v1"),
            ["openai"]
        );
    }

    #[test]
    fn test_map_provider_type_to_registry_ids() {
        assert_eq!(
            ModelRegistryService::map_provider_type_to_registry_ids(
                ApiProviderType::AnthropicCompatible
            ),
            ["anthropic"]
        );
        assert_eq!(
            ModelRegistryService::map_provider_type_to_registry_ids(ApiProviderType::Gemini),
            ["google"]
        );
    }
}
