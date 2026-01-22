//! 更新检查命令模块
//!
//! 提供自动更新检查相关的 Tauri 命令
//!
//! input: 前端调用请求
//! output: 更新信息、配置操作结果
//! pos: commands 层，被前端调用

use crate::app::AppState;
use crate::config;
use crate::services::update_check_service::{
    UpdateCheckService, UpdateCheckServiceState, UpdateInfo,
};
use crate::services::update_window;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;

/// 更新检查配置（前端可见）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckSettings {
    pub enabled: bool,
    pub check_interval_hours: u32,
    pub show_notification: bool,
    pub last_check_timestamp: u64,
    pub skipped_version: Option<String>,
}

/// 手动检查更新
#[tauri::command]
pub async fn check_update(
    update_service: State<'_, UpdateCheckServiceState>,
) -> Result<UpdateInfo, String> {
    let service = update_service.0.read().await;
    Ok(service.check_for_updates().await)
}

/// 获取更新检查配置
#[tauri::command]
pub async fn get_update_check_settings(
    app_state: State<'_, AppState>,
) -> Result<UpdateCheckSettings, String> {
    let state = app_state.read().await;
    let update_config = &state.config.experimental.update_check;

    Ok(UpdateCheckSettings {
        enabled: update_config.enabled,
        check_interval_hours: update_config.check_interval_hours,
        show_notification: update_config.show_notification,
        last_check_timestamp: update_config.last_check_timestamp,
        skipped_version: update_config.skipped_version.clone(),
    })
}

/// 更新检查配置
#[tauri::command]
pub async fn set_update_check_settings(
    app_state: State<'_, AppState>,
    settings: UpdateCheckSettings,
) -> Result<(), String> {
    let mut state = app_state.write().await;
    let update_config = &mut state.config.experimental.update_check;

    update_config.enabled = settings.enabled;
    update_config.check_interval_hours = settings.check_interval_hours;
    update_config.show_notification = settings.show_notification;
    update_config.skipped_version = settings.skipped_version;

    config::save_config(&state.config).map_err(|e| format!("保存配置失败: {}", e))
}

/// 跳过指定版本
#[tauri::command]
pub async fn skip_update_version(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    version: String,
) -> Result<(), String> {
    let mut state = app_state.write().await;
    state.config.experimental.update_check.skipped_version = Some(version);

    config::save_config(&state.config).map_err(|e| format!("保存配置失败: {}", e))?;

    // 关闭更新窗口
    let _ = update_window::close_update_window(&app_handle);

    Ok(())
}

/// 关闭更新提醒窗口
#[tauri::command]
pub fn close_update_window(app_handle: AppHandle) -> Result<(), String> {
    update_window::close_update_window(&app_handle).map_err(|e| format!("关闭更新窗口失败: {}", e))
}

/// 测试更新提醒窗口（仅开发环境使用）
#[tauri::command]
pub fn test_update_window(app_handle: AppHandle) -> Result<(), String> {
    let current_version = env!("CARGO_PKG_VERSION");
    let test_info = UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: Some("0.99.0".to_string()),
        has_update: true,
        download_url: Some(format!(
            "https://github.com/aiclientproxy/proxycast/releases/tag/v0.99.0"
        )),
        release_notes_url: None,
        checked_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        error: None,
    };

    update_window::open_update_window(&app_handle, &test_info)
        .map_err(|e| format!("打开更新窗口失败: {}", e))
}

/// 更新上次检查时间
#[tauri::command]
pub async fn update_last_check_timestamp(app_state: State<'_, AppState>) -> Result<u64, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut state = app_state.write().await;
    state.config.experimental.update_check.last_check_timestamp = now;

    config::save_config(&state.config).map_err(|e| format!("保存配置失败: {}", e))?;

    Ok(now)
}

/// 启动后台更新检查任务
///
/// 在应用启动时调用，根据配置定期检查更新
pub async fn start_background_update_check(
    app_handle: tauri::AppHandle,
    update_service: Arc<RwLock<UpdateCheckService>>,
) {
    let app_handle_clone = app_handle.clone();

    tokio::spawn(async move {
        // 延迟 30 秒后开始第一次检查，避免影响启动性能
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        loop {
            // 获取当前配置
            let (enabled, interval_hours, show_notification, last_check, skipped_version) = {
                if let Some(app_state) = app_handle_clone.try_state::<AppState>() {
                    let state = app_state.read().await;
                    let update_config = &state.config.experimental.update_check;
                    (
                        update_config.enabled,
                        update_config.check_interval_hours,
                        update_config.show_notification,
                        update_config.last_check_timestamp,
                        update_config.skipped_version.clone(),
                    )
                } else {
                    (true, 24, true, 0, None)
                }
            };

            if !enabled {
                // 如果禁用了自动检查，每小时检查一次配置是否变化
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                continue;
            }

            // 检查是否需要执行更新检查
            let service = update_service.read().await;
            let last_result = service.get_state().await.last_result;
            let latest_version = last_result
                .as_ref()
                .and_then(|r| r.latest_version.as_deref());

            if UpdateCheckService::should_check(
                last_check,
                interval_hours,
                skipped_version.as_deref(),
                latest_version,
            ) {
                drop(service);

                // 执行更新检查
                let service = update_service.read().await;
                let result = service.check_for_updates().await;

                tracing::info!(
                    "[更新检查] 当前版本: {}, 最新版本: {:?}, 有更新: {}",
                    result.current_version,
                    result.latest_version,
                    result.has_update
                );

                // 更新检查时间
                if let Some(app_state) = app_handle_clone.try_state::<AppState>() {
                    let mut state = app_state.write().await;
                    state.config.experimental.update_check.last_check_timestamp = result.checked_at;
                    let _ = config::save_config(&state.config);
                }

                // 如果有更新且启用了通知，打开独立的更新提醒窗口
                if result.has_update && show_notification {
                    // 检查是否跳过了此版本
                    let should_notify = result.latest_version.as_ref().map_or(true, |latest| {
                        skipped_version
                            .as_ref()
                            .map_or(true, |skipped| skipped != latest)
                    });

                    if should_notify {
                        // 打开独立的更新提醒窗口 - 必须在主线程执行
                        let app_handle_for_ui = app_handle_clone.clone();
                        let result_clone = result.clone();
                        let _ = app_handle_clone.run_on_main_thread(move || {
                            if let Err(e) =
                                update_window::open_update_window(&app_handle_for_ui, &result_clone)
                            {
                                tracing::error!("[更新检查] 打开更新窗口失败: {}", e);
                            }
                        });
                    }
                }
            }

            // 每小时检查一次是否需要执行更新检查
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });
}
