//! Webview 管理命令
//!
//! 提供创建和管理独立浏览器窗口的功能。
//! 使用 Tauri 2.x 的 WebviewWindow 创建独立的浏览器窗口。
//!
//! ## 功能
//! - 创建独立的浏览器窗口显示外部 URL
//! - 管理窗口生命周期
//! - 控制窗口位置和大小

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::RwLock;

/// Webview 面板信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewPanelInfo {
    /// 面板 ID
    pub id: String,
    /// 当前 URL
    pub url: String,
    /// 面板标题
    pub title: String,
    /// X 坐标
    pub x: f64,
    /// Y 坐标
    pub y: f64,
    /// 宽度
    pub width: f64,
    /// 高度
    pub height: f64,
}

/// Webview 管理器状态
pub struct WebviewManagerState {
    /// 活跃的 webview 面板
    panels: HashMap<String, WebviewPanelInfo>,
}

impl WebviewManagerState {
    pub fn new() -> Self {
        Self {
            panels: HashMap::new(),
        }
    }
}

impl Default for WebviewManagerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Webview 管理器状态包装
pub struct WebviewManagerWrapper(pub Arc<RwLock<WebviewManagerState>>);

/// 创建嵌入式 webview 的请求参数
#[derive(Debug, Deserialize)]
pub struct CreateWebviewRequest {
    /// 面板 ID（唯一标识）
    pub panel_id: String,
    /// 要加载的 URL
    pub url: String,
    /// 面板标题
    pub title: Option<String>,
    /// X 坐标（相对于主窗口）
    pub x: f64,
    /// Y 坐标（相对于主窗口）
    pub y: f64,
    /// 宽度
    pub width: f64,
    /// 高度
    pub height: f64,
}

/// 创建 webview 面板的响应
#[derive(Debug, Serialize)]
pub struct CreateWebviewResponse {
    /// 是否成功
    pub success: bool,
    /// 面板 ID
    pub panel_id: String,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 创建独立的浏览器窗口
///
/// 使用 Tauri 2.x 的 WebviewWindow 创建独立的浏览器窗口。
#[tauri::command]
pub async fn create_webview_panel(
    app: AppHandle,
    state: tauri::State<'_, WebviewManagerWrapper>,
    request: CreateWebviewRequest,
) -> Result<CreateWebviewResponse, String> {
    let panel_id = request.panel_id.clone();
    let url = request.url.clone();
    let title = request.title.unwrap_or_else(|| "Web Browser".to_string());

    tracing::info!(
        "[Webview] 创建独立窗口: id={}, url={}, size={}x{}",
        panel_id,
        url,
        request.width,
        request.height
    );

    // 检查是否已存在同 ID 的窗口
    {
        let manager = state.0.read().await;
        if manager.panels.contains_key(&panel_id) {
            return Ok(CreateWebviewResponse {
                success: false,
                panel_id,
                error: Some("窗口已存在".to_string()),
            });
        }
    }

    // 解析 URL
    let webview_url = match url.parse::<url::Url>() {
        Ok(parsed_url) => WebviewUrl::External(parsed_url),
        Err(e) => {
            return Ok(CreateWebviewResponse {
                success: false,
                panel_id,
                error: Some(format!("无效的 URL: {}", e)),
            });
        }
    };

    // 创建独立的 WebviewWindow
    match WebviewWindowBuilder::new(&app, &panel_id, webview_url)
        .title(&title)
        .inner_size(request.width, request.height)
        .min_inner_size(400.0, 300.0)
        .resizable(true)
        .center()
        .build()
    {
        Ok(_window) => {
            // 记录窗口信息
            let mut manager = state.0.write().await;
            manager.panels.insert(
                panel_id.clone(),
                WebviewPanelInfo {
                    id: panel_id.clone(),
                    url,
                    title,
                    x: 0.0,
                    y: 0.0,
                    width: request.width,
                    height: request.height,
                },
            );

            tracing::info!("[Webview] 独立窗口创建成功: {}", panel_id);

            Ok(CreateWebviewResponse {
                success: true,
                panel_id,
                error: None,
            })
        }
        Err(e) => {
            tracing::error!("[Webview] 创建独立窗口失败: {}", e);
            Ok(CreateWebviewResponse {
                success: false,
                panel_id,
                error: Some(format!("创建窗口失败: {}", e)),
            })
        }
    }
}

/// 关闭浏览器窗口
#[tauri::command]
pub async fn close_webview_panel(
    app: AppHandle,
    state: tauri::State<'_, WebviewManagerWrapper>,
    panel_id: String,
) -> Result<bool, String> {
    tracing::info!("[Webview] 尝试关闭窗口: {}", panel_id);

    // 获取并关闭窗口
    match app.get_webview_window(&panel_id) {
        Some(window) => {
            tracing::info!("[Webview] 找到窗口: {}", panel_id);

            // 关闭窗口
            match window.close() {
                Ok(_) => {
                    tracing::info!("[Webview] 窗口已关闭: {}", panel_id);
                }
                Err(e) => {
                    tracing::error!("[Webview] 关闭窗口失败: {}", e);
                }
            }
        }
        None => {
            tracing::warn!("[Webview] 未找到窗口: {}", panel_id);
        }
    }

    // 从状态中移除
    let mut manager = state.0.write().await;
    manager.panels.remove(&panel_id);

    tracing::info!("[Webview] 窗口已从状态中移除: {}", panel_id);
    Ok(true)
}

/// 导航到新 URL
#[tauri::command]
pub async fn navigate_webview_panel(
    app: AppHandle,
    state: tauri::State<'_, WebviewManagerWrapper>,
    panel_id: String,
    url: String,
) -> Result<bool, String> {
    tracing::info!("[Webview] 导航窗口 {} 到: {}", panel_id, url);

    // 解析 URL
    let parsed_url = url
        .parse::<url::Url>()
        .map_err(|e| format!("无效的 URL: {}", e))?;

    // 获取窗口并导航
    if let Some(window) = app.get_webview_window(&panel_id) {
        // 使用 eval 来导航
        let js = format!("window.location.href = '{}';", parsed_url);
        window.eval(&js).map_err(|e| format!("导航失败: {}", e))?;

        // 更新状态中的 URL
        let mut manager = state.0.write().await;
        if let Some(panel) = manager.panels.get_mut(&panel_id) {
            panel.url = url;
        }

        Ok(true)
    } else {
        Err(format!("窗口不存在: {}", panel_id))
    }
}

/// 调整窗口大小（独立窗口不需要位置参数）
#[tauri::command]
pub async fn resize_webview_panel(
    app: AppHandle,
    state: tauri::State<'_, WebviewManagerWrapper>,
    panel_id: String,
    _x: f64,
    _y: f64,
    width: f64,
    height: f64,
) -> Result<bool, String> {
    tracing::info!(
        "[Webview] 调整窗口 {} 大小: size={}x{}",
        panel_id,
        width,
        height
    );

    // 获取窗口
    if let Some(window) = app.get_webview_window(&panel_id) {
        // 设置大小
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e| format!("设置大小失败: {}", e))?;

        // 更新状态
        let mut manager = state.0.write().await;
        if let Some(panel) = manager.panels.get_mut(&panel_id) {
            panel.width = width;
            panel.height = height;
        }

        Ok(true)
    } else {
        Err(format!("窗口不存在: {}", panel_id))
    }
}

/// 获取所有活跃的浏览器窗口
#[tauri::command]
pub async fn get_webview_panels(
    state: tauri::State<'_, WebviewManagerWrapper>,
) -> Result<Vec<WebviewPanelInfo>, String> {
    let manager = state.0.read().await;
    Ok(manager.panels.values().cloned().collect())
}

/// 聚焦指定的浏览器窗口
#[tauri::command]
pub async fn focus_webview_panel(app: AppHandle, panel_id: String) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window(&panel_id) {
        window.set_focus().map_err(|e| format!("聚焦失败: {}", e))?;
        Ok(true)
    } else {
        Err(format!("窗口不存在: {}", panel_id))
    }
}
