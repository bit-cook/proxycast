//! 插件系统相关命令
//!
//! 提供插件管理和 UI 相关的 Tauri 命令：
//! - get_plugin_status: 获取插件服务状态
//! - get_plugins: 获取所有插件列表
//! - get_plugins_with_ui: 获取带有 UI 配置的已安装插件列表
//! - get_plugin_ui: 获取插件 UI 定义
//! - handle_plugin_action: 处理插件 UI 操作
//!
//! _需求: 3.1, 3.2, 3.3_

#![allow(dead_code)]

use proxycast_core::plugin::{
    PluginConfig, PluginInfo, PluginManager, PluginManifest, PluginQueueStats, PluginTaskRecord,
    PluginTaskState, PluginType,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::plugin_install_cmd::PluginInstallerState;

/// 前端调试日志命令
#[tauri::command]
pub fn frontend_debug_log(message: String) {
    println!("[Frontend] {message}");
}

/// 插件管理器状态
pub struct PluginManagerState(pub Arc<RwLock<PluginManager>>);

/// 插件状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginServiceStatus {
    pub enabled: bool,
    pub plugin_count: usize,
    pub plugins_dir: String,
}

/// 插件配置请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigRequest {
    pub enabled: bool,
    pub timeout_ms: u64,
    pub settings: serde_json::Value,
}

/// 获取插件服务状态
#[tauri::command]
pub async fn get_plugin_status(
    state: tauri::State<'_, PluginManagerState>,
) -> Result<PluginServiceStatus, String> {
    let manager = state.0.read().await;
    Ok(PluginServiceStatus {
        enabled: true,
        plugin_count: manager.count(),
        plugins_dir: manager.plugins_dir().to_string_lossy().to_string(),
    })
}

/// 获取所有插件列表
#[tauri::command]
pub async fn get_plugins(
    state: tauri::State<'_, PluginManagerState>,
) -> Result<Vec<PluginInfo>, String> {
    let manager = state.0.read().await;
    Ok(manager.list().await)
}

/// 获取单个插件信息
#[tauri::command]
pub async fn get_plugin_info(
    state: tauri::State<'_, PluginManagerState>,
    name: String,
) -> Result<Option<PluginInfo>, String> {
    let manager = state.0.read().await;
    Ok(manager.get_info(&name).await)
}

/// 启用插件
#[tauri::command]
pub async fn enable_plugin(
    state: tauri::State<'_, PluginManagerState>,
    name: String,
) -> Result<(), String> {
    let manager = state.0.read().await;
    manager.enable(&name).await.map_err(|e| e.to_string())
}

/// 禁用插件
#[tauri::command]
pub async fn disable_plugin(
    state: tauri::State<'_, PluginManagerState>,
    name: String,
) -> Result<(), String> {
    let manager = state.0.read().await;
    manager.disable(&name).await.map_err(|e| e.to_string())
}

/// 更新插件配置
#[tauri::command]
pub async fn update_plugin_config(
    state: tauri::State<'_, PluginManagerState>,
    name: String,
    config: PluginConfigRequest,
) -> Result<(), String> {
    let manager = state.0.read().await;
    let plugin_config = PluginConfig {
        enabled: config.enabled,
        timeout_ms: config.timeout_ms,
        settings: config.settings,
    };
    manager
        .update_config(&name, plugin_config)
        .await
        .map_err(|e| e.to_string())
}

/// 获取插件配置
#[tauri::command]
pub async fn get_plugin_config(
    state: tauri::State<'_, PluginManagerState>,
    name: String,
) -> Result<Option<PluginConfig>, String> {
    let manager = state.0.read().await;
    Ok(manager.get_config(&name))
}

/// 重新加载所有插件
#[tauri::command]
pub async fn reload_plugins(
    state: tauri::State<'_, PluginManagerState>,
) -> Result<Vec<String>, String> {
    let manager = state.0.read().await;
    manager.load_all().await.map_err(|e| e.to_string())
}

/// 卸载插件
#[tauri::command]
pub async fn unload_plugin(
    state: tauri::State<'_, PluginManagerState>,
    name: String,
) -> Result<(), String> {
    let manager = state.0.read().await;
    manager.unload(&name).await.map_err(|e| e.to_string())
}

/// 获取插件目录路径
#[tauri::command]
pub async fn get_plugins_dir(
    state: tauri::State<'_, PluginManagerState>,
) -> Result<String, String> {
    let manager = state.0.read().await;
    let dir = manager.plugins_dir().to_string_lossy().to_string();
    println!("[get_plugins_dir] 返回: {dir}");
    Ok(dir)
}

fn parse_task_state(state: Option<String>) -> Result<Option<PluginTaskState>, String> {
    let Some(state) = state else {
        return Ok(None);
    };
    state
        .parse::<PluginTaskState>()
        .map(Some)
        .map_err(|e| format!("解析任务状态失败: {e}"))
}

/// 查询插件任务列表
#[tauri::command]
pub async fn list_plugin_tasks(
    state: tauri::State<'_, PluginManagerState>,
    plugin_id: Option<String>,
    task_state: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<PluginTaskRecord>, String> {
    let manager = state.0.read().await;
    let parsed_state = parse_task_state(task_state)?;
    Ok(manager.list_tasks(plugin_id.as_deref(), parsed_state, limit.unwrap_or(100)))
}

/// 获取单个插件任务
#[tauri::command]
pub async fn get_plugin_task(
    state: tauri::State<'_, PluginManagerState>,
    task_id: String,
) -> Result<Option<PluginTaskRecord>, String> {
    let manager = state.0.read().await;
    Ok(manager.get_task(&task_id))
}

/// 取消插件任务
#[tauri::command]
pub async fn cancel_plugin_task(
    state: tauri::State<'_, PluginManagerState>,
    task_id: String,
) -> Result<bool, String> {
    let manager = state.0.read().await;
    Ok(manager.cancel_task(&task_id))
}

/// 获取插件队列统计
#[tauri::command]
pub async fn get_plugin_queue_stats(
    state: tauri::State<'_, PluginManagerState>,
    plugin_id: Option<String>,
) -> Result<Vec<PluginQueueStats>, String> {
    let manager = state.0.read().await;
    Ok(manager.get_queue_stats(plugin_id.as_deref()))
}

// ============================================================================
// 插件 UI 注册系统
// ============================================================================

/// 插件 UI 信息
///
/// 用于前端显示带有 UI 的插件列表
/// _需求: 3.1, 3.3_
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUIInfo {
    /// 插件 ID
    pub plugin_id: String,
    /// 插件名称
    pub name: String,
    /// 插件描述
    pub description: String,
    /// 图标名称 (Lucide 图标)
    pub icon: String,
    /// UI 展示位置列表 (如 "tools", "sidebar", "main")
    pub surfaces: Vec<String>,
}

/// 从插件目录读取 manifest 文件
///
/// 尝试读取 plugin.json 文件并解析为 PluginManifest
fn read_plugin_manifest(install_path: &Path) -> Option<PluginManifest> {
    let manifest_path = install_path.join("plugin.json");
    tracing::debug!(
        "read_plugin_manifest: install_path={:?}, manifest_path={:?}, exists={}",
        install_path,
        manifest_path,
        manifest_path.exists()
    );

    if !manifest_path.exists() {
        tracing::debug!("read_plugin_manifest: manifest file does not exist");
        return None;
    }

    match std::fs::read_to_string(&manifest_path) {
        Ok(content) => {
            tracing::debug!(
                "read_plugin_manifest: file content length={}",
                content.len()
            );
            match serde_json::from_str(&content) {
                Ok(manifest) => {
                    tracing::debug!("read_plugin_manifest: parsed successfully");
                    Some(manifest)
                }
                Err(e) => {
                    tracing::error!("read_plugin_manifest: JSON parse error: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            tracing::error!("read_plugin_manifest: read file error: {}", e);
            None
        }
    }
}

/// 获取带有 UI 配置的已安装插件列表
///
/// 从已安装插件中筛选带有 UI 配置的插件，返回 PluginUIInfo 列表
/// 同时扫描插件目录中未注册但存在的插件
/// _需求: 3.1, 3.3_
#[tauri::command]
pub async fn get_plugins_with_ui(
    installer_state: tauri::State<'_, PluginInstallerState>,
    plugin_manager_state: tauri::State<'_, PluginManagerState>,
) -> Result<Vec<PluginUIInfo>, String> {
    let installer = installer_state.0.read().await;
    let manager = plugin_manager_state.0.read().await;

    // 获取所有已安装插件（从数据库）
    let installed_plugins = installer.list_installed().map_err(|e| e.to_string())?;
    tracing::info!(
        "[get_plugins_with_ui] 从数据库获取到 {} 个已安装插件",
        installed_plugins.len()
    );
    for p in &installed_plugins {
        tracing::info!(
            "[get_plugins_with_ui] 数据库插件: id={}, install_path={:?}",
            p.id,
            p.install_path
        );
    }

    let mut registered_ids: std::collections::HashSet<String> =
        installed_plugins.iter().map(|p| p.id.clone()).collect();

    // 筛选带有 UI 配置的已注册插件
    let mut ui_plugins: Vec<PluginUIInfo> = installed_plugins
        .into_iter()
        .filter_map(|plugin| {
            // 读取插件的 manifest 文件
            let manifest = read_plugin_manifest(&plugin.install_path)?;

            // 检查是否有 UI 配置
            let ui_config = manifest.ui?;

            // 只返回有 surfaces 配置的插件
            if ui_config.surfaces.is_empty() {
                return None;
            }

            Some(PluginUIInfo {
                plugin_id: plugin.id,
                name: plugin.name,
                description: plugin.description,
                icon: ui_config.icon.unwrap_or_else(|| "puzzle".to_string()),
                surfaces: ui_config.surfaces,
            })
        })
        .collect();

    tracing::info!(
        "[get_plugins_with_ui] 从数据库筛选出 {} 个带 UI 的插件",
        ui_plugins.len()
    );

    // 扫描插件目录中未注册的插件
    let plugins_dir = manager.plugins_dir();
    tracing::info!(
        "[get_plugins_with_ui] PluginManager plugins_dir={:?}, exists={}",
        plugins_dir,
        plugins_dir.exists()
    );

    if let Ok(entries) = std::fs::read_dir(plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            tracing::debug!("[get_plugins_with_ui] 扫描目录: {:?}", path);
            if path.is_dir() {
                let plugin_id = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string());

                if let Some(id) = plugin_id {
                    tracing::info!(
                        "[get_plugins_with_ui] 发现目录: id={}, already_registered={}",
                        id,
                        registered_ids.contains(&id)
                    );

                    // 跳过已注册的插件
                    if registered_ids.contains(&id) {
                        continue;
                    }

                    // 尝试读取 manifest
                    let manifest_result = read_plugin_manifest(&path);
                    tracing::info!(
                        "[get_plugins_with_ui] 读取 manifest: id={}, success={}",
                        id,
                        manifest_result.is_some()
                    );

                    if let Some(manifest) = manifest_result {
                        tracing::info!(
                            "[get_plugins_with_ui] manifest: name={}, has_ui={}",
                            manifest.name,
                            manifest.ui.is_some()
                        );
                        if let Some(ui_config) = manifest.ui {
                            tracing::info!(
                                "[get_plugins_with_ui] ui_config: surfaces={:?}",
                                ui_config.surfaces
                            );
                            if !ui_config.surfaces.is_empty() {
                                ui_plugins.push(PluginUIInfo {
                                    plugin_id: id.clone(),
                                    name: manifest.name,
                                    description: manifest.description,
                                    icon: ui_config.icon.unwrap_or_else(|| "puzzle".to_string()),
                                    surfaces: ui_config.surfaces,
                                });
                                registered_ids.insert(id);
                            }
                        }
                    }
                }
            }
        }
    } else {
        tracing::error!("[get_plugins_with_ui] 无法读取插件目录: {:?}", plugins_dir);
    }

    tracing::info!(
        "[get_plugins_with_ui] 最终返回 {} 个带 UI 的插件: {:?}",
        ui_plugins.len(),
        ui_plugins.iter().map(|p| &p.plugin_id).collect::<Vec<_>>()
    );

    Ok(ui_plugins)
}

// ============================================================================
// 插件 UI 相关命令
// ============================================================================

use proxycast_core::plugin::{UIMessage, UserAction};

/// 获取插件 UI 定义
/// 返回插件的初始 UI 消息列表
#[tauri::command]
pub async fn get_plugin_ui(
    state: tauri::State<'_, PluginManagerState>,
    plugin_id: String,
) -> Result<Vec<UIMessage>, String> {
    let manager = state.0.read().await;

    // 获取插件的 Surface 定义
    let surfaces = manager
        .get_plugin_surfaces(&plugin_id)
        .await
        .map_err(|e| e.to_string())?;

    // 转换为 UI 消息
    let messages: Vec<UIMessage> = surfaces.into_iter().flat_map(|s| s.to_messages()).collect();

    Ok(messages)
}

/// 处理插件 UI 操作
/// 将用户操作转发给插件并返回响应消息
#[tauri::command]
pub async fn handle_plugin_action(
    state: tauri::State<'_, PluginManagerState>,
    plugin_id: String,
    action: UserAction,
) -> Result<Vec<UIMessage>, String> {
    let mut manager = state.0.write().await;

    manager
        .handle_plugin_action(&plugin_id, action)
        .await
        .map_err(|e| e.to_string())
}

/// 读取插件清单文件
///
/// 从插件目录读取 plugin.json 文件
/// 用于检查插件是否存在于文件系统中（即使未在数据库中注册）
/// 首先尝试从 PluginManager 的 plugins_dir 读取，如果失败则尝试从数据库中的 install_path 读取
#[tauri::command]
pub async fn read_plugin_manifest_cmd(
    state: tauri::State<'_, PluginManagerState>,
    installer_state: tauri::State<'_, PluginInstallerState>,
    plugin_id: String,
) -> Result<Option<PluginManifest>, String> {
    let manager = state.0.read().await;
    let plugins_dir = manager.plugins_dir();
    let plugin_path = plugins_dir.join(&plugin_id);

    // 使用 println! 确保在 release 版本中也能看到日志
    println!(
        "[read_plugin_manifest_cmd] plugin_id={}, plugins_dir={:?}, plugin_path={:?}, exists={}",
        plugin_id,
        plugins_dir,
        plugin_path,
        plugin_path.exists()
    );

    // 首先尝试从 PluginManager 的 plugins_dir 读取
    if let Some(manifest) = read_plugin_manifest(&plugin_path) {
        println!("[read_plugin_manifest_cmd] found in plugins_dir");
        println!(
            "[read_plugin_manifest_cmd] manifest: name={}, version={}, plugin_type={:?}, has_ui={}",
            manifest.name,
            manifest.version,
            manifest.plugin_type,
            manifest.ui.is_some()
        );
        // 输出序列化后的 JSON
        if let Ok(json) = serde_json::to_string(&manifest) {
            println!("[read_plugin_manifest_cmd] JSON: {json}");
        }
        return Ok(Some(manifest));
    }

    // 如果失败，尝试从数据库中的 install_path 读取
    let installer = installer_state.0.read().await;
    if let Ok(Some(installed)) = installer.get_plugin(&plugin_id) {
        println!(
            "[read_plugin_manifest_cmd] trying install_path={:?}",
            installed.install_path
        );
        if let Some(manifest) = read_plugin_manifest(&installed.install_path) {
            println!("[read_plugin_manifest_cmd] found in install_path");
            return Ok(Some(manifest));
        }
    }

    println!("[read_plugin_manifest_cmd] not found");
    Ok(None)
}

/// 启动插件 UI（用于 binary 类型插件）
///
/// 启动插件的独立 UI 窗口
#[tauri::command]
pub async fn launch_plugin_ui(
    state: tauri::State<'_, PluginManagerState>,
    installer_state: tauri::State<'_, PluginInstallerState>,
    plugin_id: String,
) -> Result<(), String> {
    let manager = state.0.read().await;
    let plugins_dir = manager.plugins_dir();
    let plugin_path = plugins_dir.join(&plugin_id);

    // 首先尝试从 PluginManager 的 plugins_dir 读取
    let (manifest, actual_plugin_path) = if let Some(m) = read_plugin_manifest(&plugin_path) {
        (m, plugin_path)
    } else {
        // 如果失败，尝试从数据库中的 install_path 读取
        let installer = installer_state.0.read().await;
        if let Ok(Some(installed)) = installer.get_plugin(&plugin_id) {
            if let Some(m) = read_plugin_manifest(&installed.install_path) {
                (m, installed.install_path.clone())
            } else {
                return Err(format!("插件 {plugin_id} 不存在"));
            }
        } else {
            return Err(format!("插件 {plugin_id} 不存在"));
        }
    };

    // 检查是否是 binary 类型
    if manifest.plugin_type != PluginType::Binary {
        return Err("只有 binary 类型的插件支持独立启动".to_string());
    }

    // 获取二进制文件路径
    let binary_config = manifest
        .binary
        .ok_or_else(|| "插件缺少 binary 配置".to_string())?;

    let binary_name = &binary_config.binary_name;
    let binary_path = actual_plugin_path.join(binary_name);

    if !binary_path.exists() {
        return Err(format!("插件二进制文件不存在: {}", binary_path.display()));
    }

    // 启动二进制文件
    std::process::Command::new(&binary_path)
        .spawn()
        .map_err(|e| format!("启动插件失败: {e}"))?;

    Ok(())
}
