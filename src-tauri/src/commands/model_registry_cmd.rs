//! 模型注册表 Tauri 命令
//!
//! 提供模型注册表相关的前端 API

use crate::models::model_registry::{
    EnhancedModelMetadata, ModelSyncState, ModelTier, ProviderAliasConfig, UserModelPreference,
};
use proxycast_services::model_registry_service::{FetchModelsResult, ModelRegistryService};
use std::collections::BTreeSet;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// 模型注册服务状态
pub type ModelRegistryState = Arc<RwLock<Option<ModelRegistryService>>>;

/// 获取所有模型
#[tauri::command]
pub async fn get_model_registry(
    state: State<'_, ModelRegistryState>,
) -> Result<Vec<EnhancedModelMetadata>, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    Ok(service.get_all_models().await)
}

/// 获取模型注册表中所有 provider_id（去重且有序）
///
/// provider_id 来源于 `src-tauri/resources/models/providers/*.json` 加载结果。
#[tauri::command]
pub async fn get_model_registry_provider_ids(
    state: State<'_, ModelRegistryState>,
) -> Result<Vec<String>, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    let models = service.get_all_models().await;
    let provider_ids: BTreeSet<String> = models.into_iter().map(|m| m.provider_id).collect();

    Ok(provider_ids.into_iter().collect())
}

/// 搜索模型
#[tauri::command]
pub async fn search_models(
    state: State<'_, ModelRegistryState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<EnhancedModelMetadata>, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    Ok(service.search_models(&query, limit.unwrap_or(50)).await)
}

/// 获取用户模型偏好
#[tauri::command]
pub async fn get_model_preferences(
    state: State<'_, ModelRegistryState>,
) -> Result<Vec<UserModelPreference>, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    service.get_all_preferences().await
}

/// 切换模型收藏状态
#[tauri::command]
pub async fn toggle_model_favorite(
    state: State<'_, ModelRegistryState>,
    model_id: String,
) -> Result<bool, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    service.toggle_favorite(&model_id).await
}

/// 隐藏模型
#[tauri::command]
pub async fn hide_model(
    state: State<'_, ModelRegistryState>,
    model_id: String,
) -> Result<(), String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    service.hide_model(&model_id).await
}

/// 记录模型使用
#[tauri::command]
pub async fn record_model_usage(
    state: State<'_, ModelRegistryState>,
    model_id: String,
) -> Result<(), String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    service.record_usage(&model_id).await
}

/// 获取模型同步状态
#[tauri::command]
pub async fn get_model_sync_state(
    state: State<'_, ModelRegistryState>,
) -> Result<ModelSyncState, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    Ok(service.get_sync_state().await)
}

/// 按 Provider 获取模型
#[tauri::command]
pub async fn get_models_for_provider(
    state: State<'_, ModelRegistryState>,
    provider_id: String,
) -> Result<Vec<EnhancedModelMetadata>, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    Ok(service.get_models_by_provider(&provider_id).await)
}

/// 按服务等级获取模型
#[tauri::command]
pub async fn get_models_by_tier(
    state: State<'_, ModelRegistryState>,
    tier: String,
) -> Result<Vec<EnhancedModelMetadata>, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    let tier: ModelTier = tier
        .parse()
        .map_err(|_| format!("无效的服务等级: {tier}"))?;

    Ok(service.get_models_by_tier(tier).await)
}

/// 获取指定 Provider 的别名配置
///
/// 用于获取 Antigravity、Kiro 等中转服务的模型别名映射
#[tauri::command]
pub async fn get_provider_alias_config(
    state: State<'_, ModelRegistryState>,
    provider: String,
) -> Result<Option<ProviderAliasConfig>, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    Ok(service.get_provider_alias_config(&provider).await)
}

/// 获取所有 Provider 的别名配置
#[tauri::command]
pub async fn get_all_alias_configs(
    state: State<'_, ModelRegistryState>,
) -> Result<std::collections::HashMap<String, ProviderAliasConfig>, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    Ok(service.get_all_alias_configs().await)
}

/// 刷新模型注册表（强制从内嵌资源重新加载）
#[tauri::command]
pub async fn refresh_model_registry(state: State<'_, ModelRegistryState>) -> Result<u32, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    service.force_reload().await
}

/// 从 Provider API 获取模型列表
///
/// 调用 Provider 的 /v1/models 端点获取模型列表，
/// 如果失败则回退到本地 JSON 文件
///
/// # 参数
/// - `provider_id`: Provider ID（如 "siliconflow", "openai"）
/// - `api_host`: API 主机地址
/// - `api_key`: API Key
#[tauri::command]
pub async fn fetch_provider_models_from_api(
    state: State<'_, ModelRegistryState>,
    provider_id: String,
    api_host: String,
    api_key: String,
) -> Result<FetchModelsResult, String> {
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    service
        .fetch_models_from_api(&provider_id, &api_host, &api_key)
        .await
}

/// 从 Provider API 获取模型列表（自动获取 API Key）
///
/// 自动从数据库获取 Provider 的 API Key，然后调用 /v1/models 端点
///
/// # 参数
/// - `provider_id`: Provider ID（如 "siliconflow", "openai"）
#[tauri::command]
pub async fn fetch_provider_models_auto(
    state: State<'_, ModelRegistryState>,
    db: tauri::State<'_, crate::database::DbConnection>,
    api_key_service: tauri::State<
        '_,
        crate::commands::api_key_provider_cmd::ApiKeyProviderServiceState,
    >,
    provider_id: String,
) -> Result<FetchModelsResult, String> {
    // 获取 Provider 信息
    let provider = api_key_service
        .0
        .get_provider(&db, &provider_id)?
        .ok_or_else(|| format!("Provider 不存在: {provider_id}"))?;

    // 获取 API Key
    let api_key = api_key_service
        .0
        .get_next_api_key(&db, &provider_id)?
        .ok_or_else(|| format!("Provider {provider_id} 没有可用的 API Key"))?;

    // 获取 API Host
    let api_host = provider.provider.api_host.clone();
    if api_host.is_empty() {
        return Err("Provider 没有配置 API Host".to_string());
    }

    // 调用模型注册服务
    let guard = state.read().await;
    let service = guard
        .as_ref()
        .ok_or_else(|| "模型注册服务未初始化".to_string())?;

    service
        .fetch_models_from_api(&provider_id, &api_host, &api_key)
        .await
}
