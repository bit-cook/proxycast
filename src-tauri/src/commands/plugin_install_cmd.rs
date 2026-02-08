//! 插件安装相关命令
//!
//! 提供插件安装、卸载和管理的 Tauri 命令：
//! - install_plugin_from_file: 从本地文件安装插件
//! - install_plugin_from_url: 从 URL 安装插件
//! - uninstall_plugin: 卸载插件
//! - list_installed_plugins: 列出已安装插件
//!
//! _需求: 1.1, 2.1, 2.2, 2.4, 3.1, 3.2, 3.3, 4.2, 6.1_

use proxycast_core::plugin::installer::{
    InstallProgress, InstalledPlugin, PluginInstaller, ProgressCallback,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::RwLock;

/// 插件安装器状态
pub struct PluginInstallerState(pub Arc<RwLock<PluginInstaller>>);

/// 安装结果响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub plugin: Option<InstalledPlugin>,
    pub error: Option<String>,
}

/// 进度事件名称
const INSTALL_PROGRESS_EVENT: &str = "plugin-install-progress";

/// Tauri 进度回调实现
///
/// 将安装进度通过 Tauri 事件发送到前端
struct TauriProgressCallback<R: Runtime> {
    app_handle: AppHandle<R>,
}

impl<R: Runtime> TauriProgressCallback<R> {
    fn new(app_handle: AppHandle<R>) -> Self {
        Self { app_handle }
    }
}

impl<R: Runtime> proxycast_core::plugin::installer::ProgressCallback for TauriProgressCallback<R> {
    fn on_progress(&self, progress: InstallProgress) {
        // 发送进度事件到前端
        let _ = self.app_handle.emit(INSTALL_PROGRESS_EVENT, &progress);
    }
}

/// 从本地文件安装插件
///
/// 流程: 验证 → 解压 → 注册 → 复制文件
/// _需求: 1.1, 3.1, 3.2, 3.3_
#[tauri::command]
pub async fn install_plugin_from_file<R: Runtime>(
    app_handle: AppHandle<R>,
    state: tauri::State<'_, PluginInstallerState>,
    file_path: String,
) -> Result<InstallResult, String> {
    let installer = state.0.read().await;
    let path = PathBuf::from(&file_path);

    // 验证文件存在
    if !path.exists() {
        return Ok(InstallResult {
            success: false,
            plugin: None,
            error: Some(format!("文件不存在: {file_path}")),
        });
    }

    // 创建进度回调
    let progress_callback = TauriProgressCallback::new(app_handle);

    // 执行安装
    match installer.install_from_file(&path, &progress_callback).await {
        Ok(plugin) => Ok(InstallResult {
            success: true,
            plugin: Some(plugin),
            error: None,
        }),
        Err(e) => {
            // 发送失败进度
            progress_callback.on_progress(InstallProgress::failed(e.to_string()));
            Ok(InstallResult {
                success: false,
                plugin: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// 从 URL 安装插件
///
/// 流程: 下载 → 验证 → 解压 → 注册 → 复制文件
/// _需求: 2.1, 2.2, 2.4_
#[tauri::command]
pub async fn install_plugin_from_url<R: Runtime>(
    app_handle: AppHandle<R>,
    state: tauri::State<'_, PluginInstallerState>,
    url: String,
) -> Result<InstallResult, String> {
    let installer = state.0.read().await;

    // 验证 URL 格式
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Ok(InstallResult {
            success: false,
            plugin: None,
            error: Some("无效的 URL 格式，必须以 http:// 或 https:// 开头".to_string()),
        });
    }

    // 创建进度回调
    let progress_callback = TauriProgressCallback::new(app_handle);

    // 执行安装
    match installer.install_from_url(&url, &progress_callback).await {
        Ok(plugin) => Ok(InstallResult {
            success: true,
            plugin: Some(plugin),
            error: None,
        }),
        Err(e) => {
            // 发送失败进度
            progress_callback.on_progress(InstallProgress::failed(e.to_string()));
            Ok(InstallResult {
                success: false,
                plugin: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// 卸载插件
///
/// 流程: 删除文件 → 注销注册表
/// _需求: 4.2_
#[tauri::command]
pub async fn uninstall_plugin(
    state: tauri::State<'_, PluginInstallerState>,
    plugin_id: String,
) -> Result<bool, String> {
    let installer = state.0.read().await;

    match installer.uninstall(&plugin_id).await {
        Ok(()) => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}

/// 列出已安装插件
///
/// _需求: 6.1_
#[tauri::command]
pub async fn list_installed_plugins(
    state: tauri::State<'_, PluginInstallerState>,
) -> Result<Vec<InstalledPlugin>, String> {
    let installer = state.0.read().await;
    installer.list_installed().map_err(|e| e.to_string())
}

/// 获取已安装插件信息
#[tauri::command]
pub async fn get_installed_plugin(
    state: tauri::State<'_, PluginInstallerState>,
    plugin_id: String,
) -> Result<Option<InstalledPlugin>, String> {
    let installer = state.0.read().await;
    installer.get_plugin(&plugin_id).map_err(|e| e.to_string())
}

/// 检查插件是否已安装
#[tauri::command]
pub async fn is_plugin_installed(
    state: tauri::State<'_, PluginInstallerState>,
    plugin_id: String,
) -> Result<bool, String> {
    let installer = state.0.read().await;
    installer
        .is_installed(&plugin_id)
        .map_err(|e| e.to_string())
}
