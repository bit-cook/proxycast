//! API Key Provider Tauri 命令
//!
//! 提供 API Key Provider 管理的前端调用接口。
//!
//! **Feature: provider-ui-refactor**
//! **Validates: Requirements 9.1**

use crate::database::dao::api_key_provider::{
    ApiKeyEntry, ApiKeyProvider, ApiProviderType, ProviderWithKeys,
};
use crate::database::DbConnection;
use crate::services::api_key_provider_service::{ApiKeyProviderService, ImportResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// API Key Provider 服务状态封装
pub struct ApiKeyProviderServiceState(pub Arc<ApiKeyProviderService>);

// ============================================================================
// 请求/响应类型
// ============================================================================

/// 添加自定义 Provider 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCustomProviderRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub api_host: String,
    pub api_version: Option<String>,
    pub project: Option<String>,
    pub location: Option<String>,
    pub region: Option<String>,
}

/// 更新 Provider 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub api_host: Option<String>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
    pub api_version: Option<String>,
    pub project: Option<String>,
    pub location: Option<String>,
    pub region: Option<String>,
}

/// 添加 API Key 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddApiKeyRequest {
    pub provider_id: String,
    pub api_key: String,
    pub alias: Option<String>,
}

/// Provider 显示数据（用于前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDisplay {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub api_host: String,
    pub is_system: bool,
    pub group: String,
    pub enabled: bool,
    pub sort_order: i32,
    pub api_version: Option<String>,
    pub project: Option<String>,
    pub location: Option<String>,
    pub region: Option<String>,
    pub api_key_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// API Key 显示数据（用于前端，掩码显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyDisplay {
    pub id: String,
    pub provider_id: String,
    /// 掩码后的 API Key
    pub api_key_masked: String,
    pub alias: Option<String>,
    pub enabled: bool,
    pub usage_count: i64,
    pub error_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

/// Provider 完整显示数据（包含 API Keys）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderWithKeysDisplay {
    #[serde(flatten)]
    pub provider: ProviderDisplay,
    pub api_keys: Vec<ApiKeyDisplay>,
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 将 API Key 转换为掩码显示
fn mask_api_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 12 {
        "****".to_string()
    } else {
        let prefix: String = chars[..6].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{}****{}", prefix, suffix)
    }
}

/// 将 ApiKeyProvider 转换为 ProviderDisplay
fn provider_to_display(provider: &ApiKeyProvider, api_key_count: usize) -> ProviderDisplay {
    ProviderDisplay {
        id: provider.id.clone(),
        name: provider.name.clone(),
        provider_type: provider.provider_type.to_string(),
        api_host: provider.api_host.clone(),
        is_system: provider.is_system,
        group: provider.group.to_string(),
        enabled: provider.enabled,
        sort_order: provider.sort_order,
        api_version: provider.api_version.clone(),
        project: provider.project.clone(),
        location: provider.location.clone(),
        region: provider.region.clone(),
        api_key_count,
        created_at: provider.created_at.to_rfc3339(),
        updated_at: provider.updated_at.to_rfc3339(),
    }
}

/// 将 ApiKeyEntry 转换为 ApiKeyDisplay（需要解密后掩码）
fn api_key_to_display(key: &ApiKeyEntry, service: &ApiKeyProviderService) -> ApiKeyDisplay {
    // 解密后掩码显示
    let masked = match service.decrypt_api_key(&key.api_key_encrypted) {
        Ok(decrypted) => mask_api_key(&decrypted),
        Err(_) => "****".to_string(),
    };

    ApiKeyDisplay {
        id: key.id.clone(),
        provider_id: key.provider_id.clone(),
        api_key_masked: masked,
        alias: key.alias.clone(),
        enabled: key.enabled,
        usage_count: key.usage_count,
        error_count: key.error_count,
        last_used_at: key.last_used_at.map(|t| t.to_rfc3339()),
        created_at: key.created_at.to_rfc3339(),
    }
}

/// 将 ProviderWithKeys 转换为 ProviderWithKeysDisplay
fn provider_with_keys_to_display(
    pwk: &ProviderWithKeys,
    service: &ApiKeyProviderService,
) -> ProviderWithKeysDisplay {
    let api_keys: Vec<ApiKeyDisplay> = pwk
        .api_keys
        .iter()
        .map(|k| api_key_to_display(k, service))
        .collect();

    ProviderWithKeysDisplay {
        provider: provider_to_display(&pwk.provider, pwk.api_keys.len()),
        api_keys,
    }
}

// ============================================================================
// Tauri 命令
// ============================================================================

/// 获取所有 API Key Provider（包含 API Keys）
#[tauri::command]
pub fn get_api_key_providers(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
) -> Result<Vec<ProviderWithKeysDisplay>, String> {
    let providers = service.0.get_all_providers(&db)?;
    Ok(providers
        .iter()
        .map(|p| provider_with_keys_to_display(p, &service.0))
        .collect())
}

/// 获取单个 API Key Provider（包含 API Keys）
#[tauri::command]
pub fn get_api_key_provider(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    id: String,
) -> Result<Option<ProviderWithKeysDisplay>, String> {
    let provider = service.0.get_provider(&db, &id)?;
    Ok(provider.map(|p| provider_with_keys_to_display(&p, &service.0)))
}

/// 添加自定义 Provider
#[tauri::command]
pub fn add_custom_api_key_provider(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    request: AddCustomProviderRequest,
) -> Result<ProviderDisplay, String> {
    let provider_type: ApiProviderType = request
        .provider_type
        .parse()
        .map_err(|e: String| format!("无效的 Provider 类型: {}", e))?;

    let provider = service.0.add_custom_provider(
        &db,
        request.name,
        provider_type,
        request.api_host,
        request.api_version,
        request.project,
        request.location,
        request.region,
    )?;

    Ok(provider_to_display(&provider, 0))
}

/// 更新 Provider 配置
#[tauri::command]
pub fn update_api_key_provider(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    id: String,
    request: UpdateProviderRequest,
) -> Result<ProviderDisplay, String> {
    let provider = service.0.update_provider(
        &db,
        &id,
        request.name,
        request.api_host,
        request.enabled,
        request.sort_order,
        request.api_version,
        request.project,
        request.location,
        request.region,
    )?;

    // 获取 API Key 数量
    let full_provider = service.0.get_provider(&db, &id)?;
    let api_key_count = full_provider.map(|p| p.api_keys.len()).unwrap_or(0);

    Ok(provider_to_display(&provider, api_key_count))
}

/// 删除自定义 Provider
#[tauri::command]
pub fn delete_custom_api_key_provider(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    id: String,
) -> Result<bool, String> {
    service.0.delete_custom_provider(&db, &id)
}

/// 添加 API Key
#[tauri::command]
pub fn add_api_key(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    request: AddApiKeyRequest,
) -> Result<ApiKeyDisplay, String> {
    let key = service
        .0
        .add_api_key(&db, &request.provider_id, &request.api_key, request.alias)?;

    Ok(api_key_to_display(&key, &service.0))
}

/// 删除 API Key
#[tauri::command]
pub fn delete_api_key(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    key_id: String,
) -> Result<bool, String> {
    service.0.delete_api_key(&db, &key_id)
}

/// 切换 API Key 启用状态
#[tauri::command]
pub fn toggle_api_key(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    key_id: String,
    enabled: bool,
) -> Result<ApiKeyDisplay, String> {
    let key = service.0.toggle_api_key(&db, &key_id, enabled)?;
    Ok(api_key_to_display(&key, &service.0))
}

/// 更新 API Key 别名
#[tauri::command]
pub fn update_api_key_alias(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    key_id: String,
    alias: Option<String>,
) -> Result<ApiKeyDisplay, String> {
    let key = service.0.update_api_key_alias(&db, &key_id, alias)?;
    Ok(api_key_to_display(&key, &service.0))
}

/// 获取下一个可用的 API Key（用于 API 调用）
#[tauri::command]
pub fn get_next_api_key(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    provider_id: String,
) -> Result<Option<String>, String> {
    service.0.get_next_api_key(&db, &provider_id)
}

/// 记录 API Key 使用
#[tauri::command]
pub fn record_api_key_usage(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    key_id: String,
) -> Result<(), String> {
    service.0.record_usage(&db, &key_id)
}

/// 记录 API Key 错误
#[tauri::command]
pub fn record_api_key_error(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    key_id: String,
) -> Result<(), String> {
    service.0.record_error(&db, &key_id)
}

/// 获取 UI 状态
#[tauri::command]
pub fn get_provider_ui_state(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    key: String,
) -> Result<Option<String>, String> {
    service.0.get_ui_state(&db, &key)
}

/// 设置 UI 状态
#[tauri::command]
pub fn set_provider_ui_state(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    key: String,
    value: String,
) -> Result<(), String> {
    service.0.set_ui_state(&db, &key, &value)
}

/// 批量更新 Provider 排序顺序
/// **Validates: Requirements 8.4**
#[tauri::command]
pub fn update_provider_sort_orders(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    sort_orders: Vec<(String, i32)>,
) -> Result<(), String> {
    service.0.update_provider_sort_orders(&db, sort_orders)
}

/// 导出 Provider 配置
#[tauri::command]
pub fn export_api_key_providers(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    include_keys: bool,
) -> Result<String, String> {
    let config = service.0.export_config(&db, include_keys)?;
    serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失败: {}", e))
}

/// 导入 Provider 配置
#[tauri::command]
pub fn import_api_key_providers(
    db: State<'_, DbConnection>,
    service: State<'_, ApiKeyProviderServiceState>,
    config_json: String,
) -> Result<ImportResult, String> {
    service.0.import_config(&db, &config_json)
}
