use crate::database::DbConnection;
use crate::models::{AppType, Provider};
use crate::services::live_sync::{check_config_sync, sync_from_external, SyncCheckResult};
use crate::services::switch::SwitchService;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn get_switch_providers(
    db: State<'_, DbConnection>,
    app_type: String,
) -> Result<Vec<Provider>, String> {
    SwitchService::get_providers(&db, &app_type)
}

#[tauri::command]
pub fn get_current_switch_provider(
    db: State<'_, DbConnection>,
    app_type: String,
) -> Result<Option<Provider>, String> {
    SwitchService::get_current_provider(&db, &app_type)
}

#[tauri::command]
pub fn add_switch_provider(db: State<'_, DbConnection>, provider: Provider) -> Result<(), String> {
    SwitchService::add_provider(&db, provider)
}

#[tauri::command]
pub fn update_switch_provider(
    db: State<'_, DbConnection>,
    provider: Provider,
) -> Result<(), String> {
    SwitchService::update_provider(&db, provider)
}

#[tauri::command]
pub fn delete_switch_provider(
    db: State<'_, DbConnection>,
    app_type: String,
    id: String,
) -> Result<(), String> {
    SwitchService::delete_provider(&db, &app_type, &id)
}

#[tauri::command]
pub fn switch_provider(
    db: State<'_, DbConnection>,
    app_type: String,
    id: String,
) -> Result<(), String> {
    SwitchService::switch_provider(&db, &app_type, &id)
}

#[tauri::command]
pub fn import_default_config(
    db: State<'_, DbConnection>,
    app_type: String,
) -> Result<bool, String> {
    SwitchService::import_default_config(&db, &app_type)
}

#[tauri::command]
pub fn read_live_provider_settings(app_type: String) -> Result<Value, String> {
    SwitchService::read_live_settings(&app_type)
}

/// 检查配置同步状态
#[tauri::command]
pub fn check_config_sync_status(
    db: State<'_, DbConnection>,
    app_type: String,
) -> Result<SyncCheckResult, String> {
    // 解析 app_type
    let app_type_enum: AppType = app_type
        .parse()
        .map_err(|e| format!("Invalid app type: {}", e))?;

    // 获取当前 ProxyCast 中设置的 provider
    let current_provider = SwitchService::get_current_provider(&db, &app_type)?
        .map(|p| p.id)
        .unwrap_or_else(|| "unknown".to_string());

    // 检查同步状态
    check_config_sync(&app_type_enum, &current_provider)
        .map_err(|e| format!("Failed to check config sync: {}", e))
}

/// 从外部配置同步到 ProxyCast
#[tauri::command]
pub fn sync_from_external_config(
    db: State<'_, DbConnection>,
    app_type: String,
) -> Result<String, String> {
    // 解析 app_type
    let app_type_enum: AppType = app_type
        .parse()
        .map_err(|e| format!("Invalid app type: {}", e))?;

    // 从外部配置获取 provider
    let external_provider = sync_from_external(&app_type_enum)
        .map_err(|e| format!("Failed to sync from external: {}", e))?;

    // 切换到外部检测到的 provider
    SwitchService::switch_provider(&db, &app_type, &external_provider)?;

    Ok(format!(
        "已同步到外部配置的 provider: {}",
        external_provider
    ))
}
