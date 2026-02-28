//! Webview 管理命令
//!
//! 提供创建和管理独立浏览器窗口的功能。
//! 使用 Tauri 2.x 的 WebviewWindow 创建独立的浏览器窗口。
//!
//! ## 功能
//! - 创建独立的浏览器窗口显示外部 URL
//! - 管理窗口生命周期
//! - 控制窗口位置和大小

use crate::app::AppState;
use aster::chrome_mcp::{
    get_chrome_mcp_tools, is_chrome_integration_configured, is_chrome_integration_supported,
};
use once_cell::sync::Lazy;
use proxycast_server::chrome_bridge::{
    self, ChromeBridgeCommandRequest, ChromeBridgeCommandResult, ChromeBridgeStatusSnapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::{Mutex, RwLock};

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

/// Chrome Profile 进程内部状态
struct ChromeProfileProcess {
    profile_key: String,
    browser_source: String,
    browser_path: String,
    profile_dir: String,
    remote_debugging_port: u16,
    started_at: String,
    last_url: String,
    child: Child,
}

impl ChromeProfileProcess {
    fn as_info(&self) -> ChromeProfileSessionInfo {
        ChromeProfileSessionInfo {
            profile_key: self.profile_key.clone(),
            browser_source: self.browser_source.clone(),
            browser_path: self.browser_path.clone(),
            profile_dir: self.profile_dir.clone(),
            remote_debugging_port: self.remote_debugging_port,
            pid: self.child.id(),
            started_at: self.started_at.clone(),
            last_url: self.last_url.clone(),
        }
    }
}

/// Chrome Profile 会话管理器状态
pub struct ChromeProfileManagerState {
    sessions: HashMap<String, ChromeProfileProcess>,
}

impl ChromeProfileManagerState {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }
}

impl Default for ChromeProfileManagerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Chrome Profile 管理器状态包装
pub struct ChromeProfileManagerWrapper(pub Arc<Mutex<ChromeProfileManagerState>>);

static SHARED_CHROME_PROFILE_MANAGER: Lazy<Arc<Mutex<ChromeProfileManagerState>>> =
    Lazy::new(|| Arc::new(Mutex::new(ChromeProfileManagerState::new())));

pub fn shared_chrome_profile_manager() -> Arc<Mutex<ChromeProfileManagerState>> {
    SHARED_CHROME_PROFILE_MANAGER.clone()
}

/// 创建嵌入式 webview 的请求参数
#[derive(Debug, Deserialize)]
pub struct CreateWebviewRequest {
    /// 面板 ID（唯一标识）
    pub panel_id: String,
    /// 要加载的 URL
    pub url: String,
    /// 面板标题
    pub title: Option<String>,
    /// X 坐标（相对于主窗口）- 预留，当前使用居中显示
    #[allow(dead_code)]
    pub x: f64,
    /// Y 坐标（相对于主窗口）- 预留，当前使用居中显示
    #[allow(dead_code)]
    pub y: f64,
    /// 宽度
    pub width: f64,
    /// 高度
    pub height: f64,
    /// Profile 隔离键（用于区分不同站点/用途）
    #[serde(default)]
    pub profile_key: Option<String>,
    /// 是否启用持久化 profile（独立 cookies/localStorage）
    #[serde(default)]
    pub persistent_profile: bool,
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

/// 启动外部 Chrome Profile 的请求参数
#[derive(Debug, Deserialize)]
pub struct OpenChromeProfileRequest {
    /// Profile 隔离键（用于不同用途隔离）
    pub profile_key: String,
    /// 要打开的 URL
    pub url: String,
}

/// 启动外部 Chrome Profile 的响应
#[derive(Debug, Serialize)]
pub struct OpenChromeProfileResponse {
    /// 是否成功
    pub success: bool,
    /// 是否复用已有会话
    pub reused: bool,
    /// 浏览器来源：system / playwright
    pub browser_source: Option<String>,
    /// 浏览器可执行文件路径
    pub browser_path: Option<String>,
    /// Profile 数据目录
    pub profile_dir: Option<String>,
    /// Chrome 远程调试端口
    pub remote_debugging_port: Option<u16>,
    /// Chrome 进程 PID
    pub pid: Option<u32>,
    /// DevTools HTTP 端点
    pub devtools_http_url: Option<String>,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// Chrome Profile 会话信息
#[derive(Debug, Clone, Serialize)]
pub struct ChromeProfileSessionInfo {
    /// Profile 隔离键
    pub profile_key: String,
    /// 浏览器来源
    pub browser_source: String,
    /// 浏览器可执行文件路径
    pub browser_path: String,
    /// Profile 目录
    pub profile_dir: String,
    /// 远程调试端口
    pub remote_debugging_port: u16,
    /// 进程 PID
    pub pid: u32,
    /// 启动时间（RFC3339）
    pub started_at: String,
    /// 最近一次打开的 URL
    pub last_url: String,
}

/// Chrome 扩展桥接端点信息
#[derive(Debug, Clone, Serialize)]
pub struct ChromeBridgeEndpointInfo {
    /// 当前服务器是否运行
    pub server_running: bool,
    /// WebSocket 主机
    pub host: String,
    /// WebSocket 端口
    pub port: u16,
    /// observer 通道 URL
    pub observer_ws_url: String,
    /// control 通道 URL
    pub control_ws_url: String,
    /// Bridge Key（与 server.api_key 一致）
    pub bridge_key: String,
}

const ASTER_CHROME_TOOL_PREFIX: &str = "mcp__proxycast-browser__";
const DEFAULT_BROWSER_ACTION_TIMEOUT_MS: u64 = 30_000;
const MIN_BROWSER_ACTION_TIMEOUT_MS: u64 = 1_000;
const MAX_BROWSER_ACTION_TIMEOUT_MS: u64 = 120_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BrowserBackendType {
    AsterCompat,
    ProxycastExtensionBridge,
    CdpDirect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrowserBackendPolicy {
    pub priority: Vec<BrowserBackendType>,
    #[serde(default = "default_browser_auto_fallback")]
    pub auto_fallback: bool,
}

fn default_browser_auto_fallback() -> bool {
    true
}

impl Default for BrowserBackendPolicy {
    fn default() -> Self {
        Self {
            priority: vec![
                BrowserBackendType::AsterCompat,
                BrowserBackendType::ProxycastExtensionBridge,
                BrowserBackendType::CdpDirect,
            ],
            auto_fallback: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserBackendStatusItem {
    pub backend: BrowserBackendType,
    pub available: bool,
    pub reason: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserBackendsStatusSnapshot {
    pub policy: BrowserBackendPolicy,
    pub bridge_observer_count: usize,
    pub bridge_control_count: usize,
    pub running_profile_count: usize,
    pub cdp_alive_profile_count: usize,
    pub aster_native_host_supported: bool,
    pub aster_native_host_configured: bool,
    pub backends: Vec<BrowserBackendStatusItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrowserActionRequest {
    #[serde(default)]
    pub profile_key: Option<String>,
    #[serde(default)]
    pub backend: Option<BrowserBackendType>,
    pub action: String,
    #[serde(default)]
    pub args: Value,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserActionAttempt {
    pub backend: BrowserBackendType,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserActionResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<BrowserBackendType>,
    pub action: String,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub attempts: Vec<BrowserActionAttempt>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CdpTargetInfo {
    id: String,
    title: String,
    url: String,
    #[serde(rename = "type")]
    target_type: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_debugger_url: Option<String>,
}

static BROWSER_BACKEND_POLICY: Lazy<RwLock<BrowserBackendPolicy>> =
    Lazy::new(|| RwLock::new(BrowserBackendPolicy::default()));

const BROWSER_AUDIT_LOG_MAX: usize = 200;

#[derive(Debug, Clone, Serialize)]
pub struct BrowserActionAuditRecord {
    pub id: String,
    pub created_at: String,
    pub action: String,
    pub profile_key: Option<String>,
    pub requested_backend: Option<BrowserBackendType>,
    pub selected_backend: Option<BrowserBackendType>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub attempts: Vec<BrowserActionAttempt>,
}

static BROWSER_ACTION_AUDIT_LOGS: Lazy<Mutex<VecDeque<BrowserActionAuditRecord>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

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

    // 解析 URL
    let parsed_url = match url.parse::<url::Url>() {
        Ok(parsed_url) => parsed_url,
        Err(e) => {
            return Ok(CreateWebviewResponse {
                success: false,
                panel_id,
                error: Some(format!("无效的 URL: {e}")),
            });
        }
    };
    let webview_url = WebviewUrl::External(parsed_url.clone());

    // 若窗口已存在，复用并导航
    if let Some(window) = app.get_webview_window(&panel_id) {
        let js_url =
            serde_json::to_string(parsed_url.as_str()).map_err(|e| format!("URL 编码失败: {e}"))?;
        let js = format!("window.location.href = {js_url};");
        if let Err(e) = window.eval(&js) {
            tracing::warn!("[Webview] 已存在窗口导航失败: {}", e);
        }
        let _ = window.set_title(&title);
        let _ = window.set_focus();

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

        tracing::info!("[Webview] 复用已存在窗口: {}", panel_id);
        return Ok(CreateWebviewResponse {
            success: true,
            panel_id,
            error: None,
        });
    }

    // 创建独立的 WebviewWindow
    let mut builder = WebviewWindowBuilder::new(&app, &panel_id, webview_url)
        .title(&title)
        .inner_size(request.width, request.height)
        .min_inner_size(400.0, 300.0)
        .resizable(true)
        .center();

    if request.persistent_profile {
        let profile_key = request.profile_key.as_deref().unwrap_or(&panel_id);
        let profile_dir = resolve_profile_data_dir(&app, profile_key)?;
        std::fs::create_dir_all(&profile_dir).map_err(|e| format!("创建 profile 目录失败: {e}"))?;
        builder = builder.data_directory(profile_dir);
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            builder = builder.data_store_identifier(profile_data_store_identifier(profile_key));
        }
    }

    match builder.build() {
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
                error: Some(format!("创建窗口失败: {e}")),
            })
        }
    }
}

/// 使用独立 profile 启动外部 Chrome 窗口
#[tauri::command]
pub async fn open_chrome_profile_window(
    app: AppHandle,
    app_state: tauri::State<'_, AppState>,
    state: tauri::State<'_, ChromeProfileManagerWrapper>,
    request: OpenChromeProfileRequest,
) -> Result<OpenChromeProfileResponse, String> {
    let profile_key = normalize_profile_key(&request.profile_key);
    let parsed_url = match request.url.parse::<url::Url>() {
        Ok(url) => url,
        Err(e) => {
            return Ok(OpenChromeProfileResponse {
                success: false,
                reused: false,
                browser_source: None,
                browser_path: None,
                profile_dir: None,
                remote_debugging_port: None,
                pid: None,
                devtools_http_url: None,
                error: Some(format!("无效的 URL: {e}")),
            });
        }
    };
    let url_text = parsed_url.to_string();

    let (browser_path, browser_source) = match get_available_chrome_path() {
        Some(v) => v,
        None => {
            return Ok(OpenChromeProfileResponse {
                success: false,
                reused: false,
                browser_source: None,
                browser_path: None,
                profile_dir: None,
                remote_debugging_port: None,
                pid: None,
                devtools_http_url: None,
                error: Some(
                    "未找到可用的 Chrome/Chromium。请安装 Google Chrome 或运行: npx playwright install chromium"
                        .to_string(),
                ),
            });
        }
    };

    let profile_dir = resolve_chrome_profile_data_dir(&app, &profile_key)?;
    std::fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("创建 Chrome profile 目录失败: {e}"))?;
    let remote_port = profile_remote_debugging_port(&profile_key);
    let devtools_http_url = format!("http://127.0.0.1:{remote_port}/json/version");

    // 准备 Chrome 扩展（获取 server 配置并生成 auto_config.json）
    let extension_dir = {
        let state_guard = app_state.read().await;
        let status = state_guard.status();
        let host = normalize_bridge_host(&status.host);
        let port = status.port;
        let bridge_key = state_guard.config.server.api_key.clone();
        let server_url = format!("ws://{host}:{port}");

        prepare_chrome_extension(&app, &profile_dir, &server_url, &bridge_key, &profile_key)?
    };

    {
        let mut manager = state.0.lock().await;
        if let Some(existing) = manager.sessions.get_mut(&profile_key) {
            match existing.child.try_wait() {
                Ok(None) => {
                    // reuse 场景：不重复加载扩展
                    spawn_chrome_with_profile(
                        &existing.browser_path,
                        Path::new(&existing.profile_dir),
                        existing.remote_debugging_port,
                        &url_text,
                        true,
                        None,
                    )?;
                    existing.last_url = url_text.clone();
                    return Ok(OpenChromeProfileResponse {
                        success: true,
                        reused: true,
                        browser_source: Some(existing.browser_source.clone()),
                        browser_path: Some(existing.browser_path.clone()),
                        profile_dir: Some(existing.profile_dir.clone()),
                        remote_debugging_port: Some(existing.remote_debugging_port),
                        pid: Some(existing.child.id()),
                        devtools_http_url: Some(format!(
                            "http://127.0.0.1:{}/json/version",
                            existing.remote_debugging_port
                        )),
                        error: None,
                    });
                }
                Ok(Some(_)) | Err(_) => {
                    manager.sessions.remove(&profile_key);
                }
            }
        }
    }

    let child = spawn_chrome_with_profile(
        &browser_path,
        &profile_dir,
        remote_port,
        &url_text,
        true,
        Some(&extension_dir),
    )?;
    let pid = child.id();

    tracing::info!(
        "[ChromeProfile] 启动浏览器: source={}, path={}, profile_key={}, pid={}, port={}",
        browser_source,
        browser_path,
        profile_key,
        pid,
        remote_port
    );

    {
        let mut manager = state.0.lock().await;
        manager.sessions.insert(
            profile_key.clone(),
            ChromeProfileProcess {
                profile_key,
                browser_source: browser_source.clone(),
                browser_path: browser_path.clone(),
                profile_dir: profile_dir.to_string_lossy().to_string(),
                remote_debugging_port: remote_port,
                started_at: chrono::Utc::now().to_rfc3339(),
                last_url: url_text,
                child,
            },
        );
    }

    Ok(OpenChromeProfileResponse {
        success: true,
        reused: false,
        browser_source: Some(browser_source),
        browser_path: Some(browser_path),
        profile_dir: Some(profile_dir.to_string_lossy().to_string()),
        remote_debugging_port: Some(remote_port),
        pid: Some(pid),
        devtools_http_url: Some(devtools_http_url),
        error: None,
    })
}

#[tauri::command]
pub async fn get_chrome_profile_sessions(
    state: tauri::State<'_, ChromeProfileManagerWrapper>,
) -> Result<Vec<ChromeProfileSessionInfo>, String> {
    let mut manager = state.0.lock().await;
    let mut stale_keys = Vec::new();
    let mut sessions = Vec::new();

    for (key, process) in &mut manager.sessions {
        match process.child.try_wait() {
            Ok(None) => sessions.push(process.as_info()),
            Ok(Some(_status)) => stale_keys.push(key.clone()),
            Err(e) => {
                tracing::warn!("[ChromeProfile] 读取进程状态失败: key={}, err={}", key, e);
                stale_keys.push(key.clone());
            }
        }
    }

    for key in stale_keys {
        manager.sessions.remove(&key);
    }

    Ok(sessions)
}

#[tauri::command]
pub async fn close_chrome_profile_session(
    state: tauri::State<'_, ChromeProfileManagerWrapper>,
    profile_key: String,
) -> Result<bool, String> {
    let key = normalize_profile_key(&profile_key);
    let mut manager = state.0.lock().await;

    if let Some(mut process) = manager.sessions.remove(&key) {
        match process.child.try_wait() {
            Ok(Some(_)) => Ok(true),
            Ok(None) => {
                if let Err(e) = process.child.kill() {
                    tracing::warn!("[ChromeProfile] 结束进程失败: key={}, err={}", key, e);
                }
                let _ = process.child.wait();
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("[ChromeProfile] 读取进程状态失败: key={}, err={}", key, e);
                Ok(true)
            }
        }
    } else {
        Ok(false)
    }
}

/// 获取 ChromeBridge 连接端点信息
#[tauri::command]
pub async fn get_chrome_bridge_endpoint_info(
    app_state: tauri::State<'_, AppState>,
) -> Result<ChromeBridgeEndpointInfo, String> {
    let state = app_state.read().await;
    let status = state.status();
    let host = normalize_bridge_host(&status.host);
    let port = status.port;
    let bridge_key = state.config.server.api_key.clone();

    Ok(ChromeBridgeEndpointInfo {
        server_running: status.running,
        observer_ws_url: format!("ws://{host}:{port}/proxycast-chrome-observer/{bridge_key}"),
        control_ws_url: format!("ws://{host}:{port}/proxycast-chrome-control/{bridge_key}"),
        host,
        port,
        bridge_key,
    })
}

/// 获取 ChromeBridge 状态快照（observer/control/pending）
#[tauri::command]
pub async fn get_chrome_bridge_status() -> Result<ChromeBridgeStatusSnapshot, String> {
    Ok(chrome_bridge::chrome_bridge_hub()
        .get_status_snapshot()
        .await)
}

/// 通过 ChromeBridge 执行命令（用于设置页测试）
#[tauri::command]
pub async fn chrome_bridge_execute_command(
    request: ChromeBridgeCommandRequest,
) -> Result<ChromeBridgeCommandResult, String> {
    chrome_bridge::chrome_bridge_hub()
        .execute_api_command(request)
        .await
}

/// 获取浏览器后端策略
#[tauri::command]
pub async fn get_browser_backend_policy() -> Result<BrowserBackendPolicy, String> {
    Ok(BROWSER_BACKEND_POLICY.read().await.clone())
}

/// 设置浏览器后端策略
#[tauri::command]
pub async fn set_browser_backend_policy(
    policy: BrowserBackendPolicy,
) -> Result<BrowserBackendPolicy, String> {
    let normalized = normalize_backend_policy(policy)?;
    {
        let mut guard = BROWSER_BACKEND_POLICY.write().await;
        *guard = normalized.clone();
    }
    Ok(normalized)
}

/// 获取浏览器后端状态快照
#[tauri::command]
pub async fn get_browser_backends_status(
    state: tauri::State<'_, ChromeProfileManagerWrapper>,
) -> Result<BrowserBackendsStatusSnapshot, String> {
    let policy = BROWSER_BACKEND_POLICY.read().await.clone();
    let bridge_status = chrome_bridge::chrome_bridge_hub()
        .get_status_snapshot()
        .await;
    let sessions = list_alive_profile_sessions(state.0.clone()).await;
    let mut cdp_alive = 0usize;
    for session in &sessions {
        if is_cdp_endpoint_alive(session.remote_debugging_port).await {
            cdp_alive += 1;
        }
    }

    let extension_available = bridge_status.observer_count > 0;
    let cdp_available = cdp_alive > 0;
    let aster_supported = is_chrome_integration_supported();
    let aster_configured = is_chrome_integration_configured().await;
    let aster_available = extension_available || cdp_available || aster_configured;

    Ok(BrowserBackendsStatusSnapshot {
        policy,
        bridge_observer_count: bridge_status.observer_count,
        bridge_control_count: bridge_status.control_count,
        running_profile_count: sessions.len(),
        cdp_alive_profile_count: cdp_alive,
        aster_native_host_supported: aster_supported,
        aster_native_host_configured: aster_configured,
        backends: vec![
            BrowserBackendStatusItem {
                backend: BrowserBackendType::AsterCompat,
                available: aster_available,
                reason: if aster_available {
                    None
                } else {
                    Some("aster 兼容层当前无可用下游连接（扩展/CDP/native-host）".to_string())
                },
                capabilities: aster_backend_capabilities(),
            },
            BrowserBackendStatusItem {
                backend: BrowserBackendType::ProxycastExtensionBridge,
                available: extension_available,
                reason: if extension_available {
                    None
                } else {
                    Some("未检测到扩展 observer 连接".to_string())
                },
                capabilities: extension_backend_capabilities(),
            },
            BrowserBackendStatusItem {
                backend: BrowserBackendType::CdpDirect,
                available: cdp_available,
                reason: if cdp_available {
                    None
                } else {
                    Some("未检测到可连接的 CDP 调试端口".to_string())
                },
                capabilities: cdp_backend_capabilities(),
            },
        ],
    })
}

/// 通过统一编排层执行浏览器动作
#[tauri::command]
pub async fn browser_execute_action(
    state: tauri::State<'_, ChromeProfileManagerWrapper>,
    request: BrowserActionRequest,
) -> Result<BrowserActionResult, String> {
    browser_execute_action_with_manager(state.0.clone(), request).await
}

/// 获取浏览器动作审计日志
#[tauri::command]
pub async fn get_browser_action_audit_logs(
    limit: Option<usize>,
) -> Result<Vec<BrowserActionAuditRecord>, String> {
    let max_count = limit
        .unwrap_or(BROWSER_AUDIT_LOG_MAX)
        .min(BROWSER_AUDIT_LOG_MAX);
    let logs = BROWSER_ACTION_AUDIT_LOGS.lock().await;
    let mut result = logs.iter().cloned().collect::<Vec<_>>();
    result.reverse();
    result.truncate(max_count);
    Ok(result)
}

/// 使用指定 profile manager 执行动作（供非 Tauri 命令入口复用）
pub async fn browser_execute_action_with_manager(
    manager: Arc<Mutex<ChromeProfileManagerState>>,
    request: BrowserActionRequest,
) -> Result<BrowserActionResult, String> {
    let action = normalize_action_name(&request.action)?;
    let request_id = format!("browser-{}", uuid::Uuid::new_v4());
    let policy = BROWSER_BACKEND_POLICY.read().await.clone();
    let candidates = build_backend_candidates(request.backend.clone(), &policy);
    let allow_fallback = request.backend.is_none() && policy.auto_fallback;
    let profile_key = request
        .profile_key
        .as_deref()
        .map(normalize_profile_key)
        .or_else(|| Some("default".to_string()));

    let mut attempts = Vec::new();
    for (idx, backend) in candidates.iter().enumerate() {
        match execute_browser_action_with_backend(
            backend.clone(),
            &action,
            request.args.clone(),
            profile_key.clone(),
            request.timeout_ms,
            manager.clone(),
        )
        .await
        {
            Ok(data) => {
                attempts.push(BrowserActionAttempt {
                    backend: backend.clone(),
                    success: true,
                    message: "执行成功".to_string(),
                });
                let result = BrowserActionResult {
                    success: true,
                    backend: Some(backend.clone()),
                    action,
                    request_id: request_id.clone(),
                    data: Some(data),
                    error: None,
                    attempts: attempts.clone(),
                };
                append_browser_action_audit(BrowserActionAuditRecord {
                    id: request_id,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    action: result.action.clone(),
                    profile_key: profile_key.clone(),
                    requested_backend: request.backend.clone(),
                    selected_backend: result.backend.clone(),
                    success: true,
                    error: None,
                    attempts,
                })
                .await;
                return Ok(result);
            }
            Err(error) => {
                attempts.push(BrowserActionAttempt {
                    backend: backend.clone(),
                    success: false,
                    message: error.clone(),
                });
                if !allow_fallback || idx + 1 >= candidates.len() {
                    let result = BrowserActionResult {
                        success: false,
                        backend: None,
                        action: action.clone(),
                        request_id: request_id.clone(),
                        data: None,
                        error: Some(error.clone()),
                        attempts: attempts.clone(),
                    };
                    append_browser_action_audit(BrowserActionAuditRecord {
                        id: request_id,
                        created_at: chrono::Utc::now().to_rfc3339(),
                        action: result.action.clone(),
                        profile_key: profile_key.clone(),
                        requested_backend: request.backend.clone(),
                        selected_backend: None,
                        success: false,
                        error: Some(error),
                        attempts,
                    })
                    .await;
                    return Ok(result);
                }
            }
        }
    }

    let result = BrowserActionResult {
        success: false,
        backend: None,
        action: action.clone(),
        request_id: request_id.clone(),
        data: None,
        error: Some("没有可用的浏览器后端".to_string()),
        attempts: attempts.clone(),
    };
    append_browser_action_audit(BrowserActionAuditRecord {
        id: request_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        action,
        profile_key,
        requested_backend: request.backend,
        selected_backend: None,
        success: false,
        error: result.error.clone(),
        attempts,
    })
    .await;
    Ok(result)
}

/// 使用全局 profile manager 执行动作（供 Agent 工具复用）
pub async fn browser_execute_action_global(
    request: BrowserActionRequest,
) -> Result<BrowserActionResult, String> {
    browser_execute_action_with_manager(shared_chrome_profile_manager(), request).await
}

async fn append_browser_action_audit(record: BrowserActionAuditRecord) {
    let mut logs = BROWSER_ACTION_AUDIT_LOGS.lock().await;
    logs.push_back(record);
    while logs.len() > BROWSER_AUDIT_LOG_MAX {
        logs.pop_front();
    }
}

fn normalize_backend_policy(policy: BrowserBackendPolicy) -> Result<BrowserBackendPolicy, String> {
    let mut priority = Vec::new();
    for backend in policy.priority {
        if !priority.contains(&backend) {
            priority.push(backend);
        }
    }
    for backend in [
        BrowserBackendType::AsterCompat,
        BrowserBackendType::ProxycastExtensionBridge,
        BrowserBackendType::CdpDirect,
    ] {
        if !priority.contains(&backend) {
            priority.push(backend);
        }
    }
    if priority.is_empty() {
        return Err("后端优先级不能为空".to_string());
    }
    Ok(BrowserBackendPolicy {
        priority,
        auto_fallback: policy.auto_fallback,
    })
}

fn build_backend_candidates(
    forced_backend: Option<BrowserBackendType>,
    policy: &BrowserBackendPolicy,
) -> Vec<BrowserBackendType> {
    if let Some(backend) = forced_backend {
        return vec![backend];
    }
    if policy.priority.is_empty() {
        return BrowserBackendPolicy::default().priority;
    }
    policy.priority.clone()
}

fn normalize_action_name(action: &str) -> Result<String, String> {
    let raw = action.trim();
    if raw.is_empty() {
        return Err("action 不能为空".to_string());
    }
    let stripped = raw
        .strip_prefix(ASTER_CHROME_TOOL_PREFIX)
        .unwrap_or(raw)
        .trim();
    if stripped.is_empty() {
        return Err("action 无效".to_string());
    }
    Ok(stripped.to_ascii_lowercase())
}

fn normalize_action_timeout(timeout_ms: Option<u64>) -> u64 {
    timeout_ms
        .unwrap_or(DEFAULT_BROWSER_ACTION_TIMEOUT_MS)
        .clamp(MIN_BROWSER_ACTION_TIMEOUT_MS, MAX_BROWSER_ACTION_TIMEOUT_MS)
}

fn aster_backend_capabilities() -> Vec<String> {
    get_chrome_mcp_tools()
        .into_iter()
        .map(|tool| tool.name)
        .collect()
}

fn extension_backend_capabilities() -> Vec<String> {
    vec![
        "navigate".to_string(),
        "read_page".to_string(),
        "get_page_text".to_string(),
        "find".to_string(),
        "computer".to_string(),
        "form_input".to_string(),
        "tabs_context_mcp".to_string(),
        "open_url".to_string(),
        "click".to_string(),
        "type".to_string(),
        "scroll".to_string(),
        "scroll_page".to_string(),
        "get_page_info".to_string(),
        "refresh_page".to_string(),
        "go_back".to_string(),
        "go_forward".to_string(),
        "switch_tab".to_string(),
    ]
}

fn cdp_backend_capabilities() -> Vec<String> {
    vec![
        "tabs_context_mcp".to_string(),
        "navigate".to_string(),
        "read_page".to_string(),
        "get_page_text".to_string(),
    ]
}

fn action_arg_string(args: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        args.get(*key)
            .and_then(Value::as_str)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    })
}

fn action_arg_bool(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn action_arg_u64(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(Value::as_u64)
}

async fn execute_browser_action_with_backend(
    backend: BrowserBackendType,
    action: &str,
    args: Value,
    profile_key: Option<String>,
    timeout_ms: Option<u64>,
    manager: Arc<Mutex<ChromeProfileManagerState>>,
) -> Result<Value, String> {
    match backend {
        BrowserBackendType::ProxycastExtensionBridge => {
            execute_extension_backend_action(action, args, profile_key, timeout_ms, manager).await
        }
        BrowserBackendType::CdpDirect => {
            execute_cdp_backend_action(action, args, profile_key, manager).await
        }
        BrowserBackendType::AsterCompat => {
            execute_aster_compat_action(action, args, profile_key, timeout_ms, manager).await
        }
    }
}

async fn execute_aster_compat_action(
    action: &str,
    args: Value,
    profile_key: Option<String>,
    timeout_ms: Option<u64>,
    manager: Arc<Mutex<ChromeProfileManagerState>>,
) -> Result<Value, String> {
    match action {
        "tabs_context_mcp" | "read_page" | "get_page_text" => {
            if let Ok(result) = execute_cdp_backend_action(
                action,
                args.clone(),
                profile_key.clone(),
                manager.clone(),
            )
            .await
            {
                return Ok(result);
            }
            execute_extension_backend_action(action, args, profile_key, timeout_ms, manager).await
        }
        "read_console_messages" | "read_network_requests" => {
            execute_cdp_backend_action(action, args, profile_key, manager).await
        }
        "tabs_create_mcp" => {
            let mut next_args = args;
            if action_arg_string(&next_args, &["url"]).is_none() {
                next_args["url"] = Value::String("about:blank".to_string());
            }
            next_args["action"] = Value::String("goto".to_string());
            execute_extension_backend_action(
                "navigate",
                next_args,
                profile_key,
                timeout_ms,
                manager,
            )
            .await
        }
        "shortcuts_list" => Ok(json!({
            "supported": false,
            "message": "当前后端尚未实现 shortcuts_list",
            "shortcuts": [],
        })),
        "update_plan" => Ok(json!({
            "accepted": true,
            "plan": action_arg_string(&args, &["plan"]).unwrap_or_default(),
        })),
        "shortcuts_execute" | "gif_creator" | "upload_image" | "resize_window"
        | "javascript_tool" => Err(format!(
            "aster 兼容层暂不支持 {action}，请切换为扩展桥接或补充实现"
        )),
        _ => execute_extension_backend_action(action, args, profile_key, timeout_ms, manager).await,
    }
}

async fn execute_extension_backend_action(
    action: &str,
    args: Value,
    profile_key: Option<String>,
    timeout_ms: Option<u64>,
    manager: Arc<Mutex<ChromeProfileManagerState>>,
) -> Result<Value, String> {
    match action {
        "navigate" => {
            let nav_action =
                action_arg_string(&args, &["action"]).unwrap_or_else(|| "goto".to_string());
            match nav_action.as_str() {
                "goto" => execute_bridge_api_command(ChromeBridgeCommandRequest {
                    profile_key,
                    command: "open_url".to_string(),
                    target: None,
                    text: None,
                    url: action_arg_string(&args, &["url"]),
                    wait_for_page_info: action_arg_bool(&args, "wait_for_page_info", true),
                    timeout_ms: Some(normalize_action_timeout(
                        action_arg_u64(&args, "timeout_ms").or(timeout_ms),
                    )),
                })
                .await
                .map(bridge_result_to_value),
                "back" => execute_bridge_api_command(ChromeBridgeCommandRequest {
                    profile_key,
                    command: "go_back".to_string(),
                    target: None,
                    text: None,
                    url: None,
                    wait_for_page_info: action_arg_bool(&args, "wait_for_page_info", true),
                    timeout_ms: Some(normalize_action_timeout(timeout_ms)),
                })
                .await
                .map(bridge_result_to_value),
                "forward" => execute_bridge_api_command(ChromeBridgeCommandRequest {
                    profile_key,
                    command: "go_forward".to_string(),
                    target: None,
                    text: None,
                    url: None,
                    wait_for_page_info: action_arg_bool(&args, "wait_for_page_info", true),
                    timeout_ms: Some(normalize_action_timeout(timeout_ms)),
                })
                .await
                .map(bridge_result_to_value),
                "reload" => execute_bridge_api_command(ChromeBridgeCommandRequest {
                    profile_key,
                    command: "refresh_page".to_string(),
                    target: None,
                    text: None,
                    url: None,
                    wait_for_page_info: action_arg_bool(&args, "wait_for_page_info", true),
                    timeout_ms: Some(normalize_action_timeout(timeout_ms)),
                })
                .await
                .map(bridge_result_to_value),
                _ => Err(format!("不支持的 navigate.action: {nav_action}")),
            }
        }
        "read_page" | "get_page_text" => execute_bridge_api_command(ChromeBridgeCommandRequest {
            profile_key,
            command: "get_page_info".to_string(),
            target: None,
            text: None,
            url: None,
            wait_for_page_info: true,
            timeout_ms: Some(normalize_action_timeout(timeout_ms)),
        })
        .await
        .map(bridge_result_to_value),
        "find" => {
            let query = action_arg_string(&args, &["query"])
                .ok_or_else(|| "find 需要 query 参数".to_string())?;
            let response = execute_bridge_api_command(ChromeBridgeCommandRequest {
                profile_key,
                command: "get_page_info".to_string(),
                target: None,
                text: None,
                url: None,
                wait_for_page_info: true,
                timeout_ms: Some(normalize_action_timeout(timeout_ms)),
            })
            .await?;
            let markdown = response
                .page_info
                .as_ref()
                .map(|v| v.markdown.clone())
                .unwrap_or_default();
            let q = query.to_ascii_lowercase();
            let matches = markdown
                .lines()
                .filter(|line| line.to_ascii_lowercase().contains(&q))
                .take(30)
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            Ok(json!({
                "query": query,
                "match_count": matches.len(),
                "matches": matches,
                "page_info": response.page_info,
            }))
        }
        "form_input" => execute_bridge_api_command(ChromeBridgeCommandRequest {
            profile_key,
            command: "type".to_string(),
            target: action_arg_string(&args, &["ref_id", "target"]),
            text: action_arg_string(&args, &["value", "text"]),
            url: None,
            wait_for_page_info: action_arg_bool(&args, "wait_for_page_info", false),
            timeout_ms: Some(normalize_action_timeout(timeout_ms)),
        })
        .await
        .map(bridge_result_to_value),
        "computer" => {
            let computer_action =
                action_arg_string(&args, &["action"]).unwrap_or_else(|| "click".to_string());
            let (command, wait_for_page_info) = match computer_action.as_str() {
                "click" => ("click".to_string(), false),
                "type" => ("type".to_string(), false),
                "scroll" => ("scroll_page".to_string(), false),
                _ => {
                    return Err(format!(
                        "扩展桥接暂不支持 computer.action={computer_action}"
                    ));
                }
            };
            let text_payload = if computer_action == "scroll" {
                let direction =
                    action_arg_string(&args, &["direction"]).unwrap_or_else(|| "down".to_string());
                let amount = action_arg_u64(&args, "amount").unwrap_or(500);
                Some(format!("{direction}:{amount}"))
            } else {
                action_arg_string(&args, &["text"])
            };
            execute_bridge_api_command(ChromeBridgeCommandRequest {
                profile_key,
                command,
                target: action_arg_string(&args, &["ref_id", "target"]),
                text: text_payload,
                url: action_arg_string(&args, &["url"]),
                wait_for_page_info: action_arg_bool(
                    &args,
                    "wait_for_page_info",
                    wait_for_page_info,
                ),
                timeout_ms: Some(normalize_action_timeout(timeout_ms)),
            })
            .await
            .map(bridge_result_to_value)
        }
        "tabs_context_mcp" => {
            let bridge_status = chrome_bridge::chrome_bridge_hub()
                .get_status_snapshot()
                .await;
            let sessions = list_alive_profile_sessions(manager).await;
            Ok(json!({
                "bridge": {
                    "observer_count": bridge_status.observer_count,
                    "control_count": bridge_status.control_count,
                    "observers": bridge_status.observers,
                },
                "profiles": sessions,
            }))
        }
        "open_url" | "click" | "type" | "scroll" | "scroll_page" | "get_page_info"
        | "refresh_page" | "go_back" | "go_forward" | "switch_tab" => {
            execute_bridge_api_command(ChromeBridgeCommandRequest {
                profile_key,
                command: action.to_string(),
                target: action_arg_string(&args, &["target", "ref_id"]),
                text: action_arg_string(&args, &["text", "value"]),
                url: action_arg_string(&args, &["url"]),
                wait_for_page_info: action_arg_bool(
                    &args,
                    "wait_for_page_info",
                    action == "get_page_info",
                ),
                timeout_ms: Some(normalize_action_timeout(timeout_ms)),
            })
            .await
            .map(bridge_result_to_value)
        }
        "read_console_messages" | "read_network_requests" => {
            Err(format!("扩展桥接暂不支持 {action}"))
        }
        _ => Err(format!("扩展桥接不支持动作: {action}")),
    }
}

async fn execute_cdp_backend_action(
    action: &str,
    args: Value,
    profile_key: Option<String>,
    manager: Arc<Mutex<ChromeProfileManagerState>>,
) -> Result<Value, String> {
    let session = select_profile_session(manager, profile_key).await?;
    if !is_cdp_endpoint_alive(session.remote_debugging_port).await {
        return Err(format!(
            "CDP 调试端口不可用: 127.0.0.1:{}",
            session.remote_debugging_port
        ));
    }

    match action {
        "tabs_context_mcp" => {
            let tabs = fetch_cdp_targets(session.remote_debugging_port).await?;
            Ok(json!({
                "profile_key": session.profile_key,
                "remote_debugging_port": session.remote_debugging_port,
                "tabs": tabs,
            }))
        }
        "navigate" => {
            let nav_action =
                action_arg_string(&args, &["action"]).unwrap_or_else(|| "goto".to_string());
            if nav_action != "goto" {
                return Err(format!(
                    "CDP 直连初版仅支持 navigate.action=goto，收到: {nav_action}"
                ));
            }
            let url = action_arg_string(&args, &["url"])
                .ok_or_else(|| "navigate 需要提供 url".to_string())?;
            let endpoint = format!(
                "http://127.0.0.1:{}/json/new?{}",
                session.remote_debugging_port,
                urlencoding::encode(&url)
            );
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
            let response = match client.put(&endpoint).send().await {
                Ok(resp) => resp,
                Err(_) => client
                    .get(&endpoint)
                    .send()
                    .await
                    .map_err(|e| format!("CDP navigate 调用失败: {e}"))?,
            };
            if !response.status().is_success() {
                return Err(format!("CDP navigate 返回失败状态: {}", response.status()));
            }
            let text = response.text().await.unwrap_or_default();
            Ok(json!({
                "profile_key": session.profile_key,
                "remote_debugging_port": session.remote_debugging_port,
                "url": url,
                "response": text,
            }))
        }
        "read_page" | "get_page_text" => {
            let tabs = fetch_cdp_targets(session.remote_debugging_port).await?;
            let current = tabs
                .iter()
                .find(|tab| tab.target_type == "page")
                .or_else(|| tabs.first())
                .cloned()
                .ok_or_else(|| "CDP 未返回可用标签页".to_string())?;
            let markdown = format!(
                "# {}\nURL: {}\n\nCDP 直连初版仅返回标签页元信息，完整 DOM/控制能力将后续补齐。",
                current.title, current.url
            );
            Ok(json!({
                "profile_key": session.profile_key,
                "tab": current,
                "markdown": markdown,
            }))
        }
        "read_console_messages" | "read_network_requests" => Err(format!(
            "CDP 直连初版暂不支持 {}，需要建立 WebSocket DevTools 会话后补齐",
            action
        )),
        _ => Err(format!("CDP 直连不支持动作: {action}")),
    }
}

async fn execute_bridge_api_command(
    request: ChromeBridgeCommandRequest,
) -> Result<ChromeBridgeCommandResult, String> {
    chrome_bridge::chrome_bridge_hub()
        .execute_api_command(request)
        .await
}

fn bridge_result_to_value(result: ChromeBridgeCommandResult) -> Value {
    json!({
        "success": result.success,
        "request_id": result.request_id,
        "command": result.command,
        "message": result.message,
        "error": result.error,
        "page_info": result.page_info,
    })
}

async fn list_alive_profile_sessions(
    manager: Arc<Mutex<ChromeProfileManagerState>>,
) -> Vec<ChromeProfileSessionInfo> {
    let mut guard = manager.lock().await;
    let mut stale_keys = Vec::new();
    let mut sessions = Vec::new();

    for (key, process) in &mut guard.sessions {
        match process.child.try_wait() {
            Ok(None) => sessions.push(process.as_info()),
            Ok(Some(_)) => stale_keys.push(key.clone()),
            Err(_) => stale_keys.push(key.clone()),
        }
    }

    for key in stale_keys {
        guard.sessions.remove(&key);
    }

    sessions
}

async fn select_profile_session(
    manager: Arc<Mutex<ChromeProfileManagerState>>,
    profile_key: Option<String>,
) -> Result<ChromeProfileSessionInfo, String> {
    let sessions = list_alive_profile_sessions(manager).await;
    if sessions.is_empty() {
        return Err("没有可用的 Chrome profile 会话，请先打开独立浏览器窗口".to_string());
    }

    if let Some(key) = profile_key {
        let normalized = normalize_profile_key(&key);
        if let Some(session) = sessions.into_iter().find(|v| v.profile_key == normalized) {
            return Ok(session);
        }
        return Err(format!("未找到 profile_key={} 的会话", normalized));
    }

    sessions
        .into_iter()
        .next()
        .ok_or_else(|| "没有可用的 Chrome profile 会话".to_string())
}

async fn fetch_cdp_targets(port: u16) -> Result<Vec<CdpTargetInfo>, String> {
    let endpoint = format!("http://127.0.0.1:{port}/json/list");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let response = client
        .get(&endpoint)
        .send()
        .await
        .map_err(|e| format!("读取 CDP 标签页失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("读取 CDP 标签页失败: {}", response.status()));
    }
    response
        .json::<Vec<CdpTargetInfo>>()
        .await
        .map_err(|e| format!("解析 CDP 标签页失败: {e}"))
}

async fn is_cdp_endpoint_alive(port: u16) -> bool {
    let endpoint = format!("http://127.0.0.1:{port}/json/version");
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };
    match client.get(endpoint).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

fn sanitize_profile_key(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn normalize_profile_key(input: &str) -> String {
    let safe_key = sanitize_profile_key(input);
    if safe_key.trim_matches('_').is_empty() {
        "default".to_string()
    } else {
        safe_key
    }
}

fn normalize_bridge_host(host: &str) -> String {
    match host.trim() {
        "" | "0.0.0.0" | "::" | "[::]" => "127.0.0.1".to_string(),
        value => value.to_string(),
    }
}

fn profile_remote_debugging_port(profile_key: &str) -> u16 {
    const BASE_PORT: u16 = 13000;
    const RANGE: u16 = 4000;

    let mut hash: u64 = 1469598103934665603;
    for byte in profile_key.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    BASE_PORT + (hash as u16 % RANGE)
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("创建目标目录失败: {e}"))?;

    for entry in std::fs::read_dir(src).map_err(|e| format!("读取源目录失败: {e}"))? {
        let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            std::fs::copy(&path, &dst_path)
                .map_err(|e| format!("复制文件失败 {:?}: {e}", file_name))?;
        }
    }
    Ok(())
}

/// 准备 Chrome 扩展（复制到 profile 目录并生成配置）
fn prepare_chrome_extension(
    app: &AppHandle,
    profile_dir: &Path,
    server_url: &str,
    bridge_key: &str,
    profile_key: &str,
) -> Result<PathBuf, String> {
    // 确定扩展源路径
    let extension_src = if cfg!(debug_assertions) {
        // 开发模式：从当前目录向上查找项目根目录
        let current_dir = std::env::current_dir().map_err(|e| format!("获取当前目录失败: {e}"))?;

        // 如果当前目录是 src-tauri，则向上一级
        let project_root = if current_dir.ends_with("src-tauri") {
            current_dir
                .parent()
                .ok_or_else(|| "无法获取项目根目录".to_string())?
                .to_path_buf()
        } else {
            current_dir
        };

        project_root.join("extensions").join("proxycast-chrome")
    } else {
        // 打包模式：使用资源目录
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|e| format!("获取资源目录失败: {e}"))?;
        resource_dir.join("extensions").join("proxycast-chrome")
    };

    if !extension_src.exists() {
        return Err(format!("扩展源目录不存在: {:?}", extension_src));
    }

    // 目标路径：profile_dir/proxycast_extension
    let extension_dst = profile_dir.join("proxycast_extension");

    // 如果目标目录已存在，先删除（确保使用最新版本）
    if extension_dst.exists() {
        std::fs::remove_dir_all(&extension_dst).map_err(|e| format!("删除旧扩展目录失败: {e}"))?;
    }

    // 复制扩展文件
    copy_dir_recursive(&extension_src, &extension_dst)?;

    // 生成 auto_config.json
    let auto_config = json!({
        "serverUrl": server_url,
        "bridgeKey": bridge_key,
        "profileKey": profile_key,
        "monitoringEnabled": true,
    });

    let config_path = extension_dst.join("auto_config.json");
    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&auto_config).unwrap(),
    )
    .map_err(|e| format!("写入 auto_config.json 失败: {e}"))?;

    tracing::info!(
        "[ChromeExtension] 扩展已准备: dst={:?}, config={:?}",
        extension_dst,
        config_path
    );

    Ok(extension_dst)
}

fn spawn_chrome_with_profile(
    browser_path: &str,
    profile_dir: &Path,
    remote_debugging_port: u16,
    url: &str,
    new_window: bool,
    extension_dir: Option<&Path>,
) -> Result<Child, String> {
    let profile_arg = format!("--user-data-dir={}", profile_dir.to_string_lossy());
    let mut cmd = Command::new(browser_path);
    cmd.arg(profile_arg)
        .arg(format!("--remote-debugging-port={remote_debugging_port}"))
        .arg("--remote-allow-origins=*")
        .arg("--no-first-run")
        .arg("--no-default-browser-check");

    // 如果提供了扩展目录，添加 --load-extension 参数
    if let Some(ext_dir) = extension_dir {
        cmd.arg(format!("--load-extension={}", ext_dir.to_string_lossy()));
    }

    if new_window {
        cmd.arg("--new-window");
    }
    cmd.arg(url);
    cmd.spawn().map_err(|e| format!("启动 Chrome 失败: {e}"))
}

fn resolve_profile_data_dir_from_base(base_dir: &Path, profile_key: &str) -> PathBuf {
    let effective_key = normalize_profile_key(profile_key);
    base_dir.join("webview_profiles").join(effective_key)
}

fn resolve_profile_data_dir(app: &AppHandle, profile_key: &str) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {e}"))?;
    Ok(resolve_profile_data_dir_from_base(&base_dir, profile_key))
}

fn resolve_chrome_profile_data_dir(app: &AppHandle, profile_key: &str) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {e}"))?;
    Ok(base_dir
        .join("chrome_profiles")
        .join(normalize_profile_key(profile_key)))
}

fn get_system_chrome_path() -> Option<String> {
    #[cfg_attr(
        not(any(target_os = "macos", target_os = "windows")),
        allow(unused_variables)
    )]
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "macos")]
    {
        let paths = [
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
            home.join("Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
        ];
        for path in paths {
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let paths = [
            PathBuf::from("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"),
            PathBuf::from("C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe"),
            home.join("AppData\\Local\\Google\\Chrome\\Application\\chrome.exe"),
        ];
        for path in paths {
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let paths = [
            PathBuf::from("/usr/bin/google-chrome"),
            PathBuf::from("/usr/bin/google-chrome-stable"),
            PathBuf::from("/usr/bin/chromium"),
            PathBuf::from("/usr/bin/chromium-browser"),
            PathBuf::from("/snap/bin/chromium"),
        ];
        for path in paths {
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }

    None
}

fn get_playwright_cache_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "macos")]
    {
        home.join("Library").join("Caches").join("ms-playwright")
    }

    #[cfg(target_os = "windows")]
    {
        home.join("AppData").join("Local").join("ms-playwright")
    }

    #[cfg(target_os = "linux")]
    {
        home.join(".cache").join("ms-playwright")
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        home.join(".cache").join("ms-playwright")
    }
}

fn get_playwright_chrome_path() -> Option<String> {
    let cache_dir = get_playwright_cache_dir();
    let entries = std::fs::read_dir(&cache_dir).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("chromium-") || name == "chromium")
                .unwrap_or(false)
        })
        .collect();

    candidates.sort_by(|a, b| b.cmp(a));

    for base in candidates {
        #[cfg(target_os = "macos")]
        let exec_path = base
            .join("chrome-mac")
            .join("Chromium.app")
            .join("Contents")
            .join("MacOS")
            .join("Chromium");

        #[cfg(target_os = "windows")]
        let exec_path = base.join("chrome-win").join("chrome.exe");

        #[cfg(target_os = "linux")]
        let exec_path = base.join("chrome-linux").join("chrome");

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        let exec_path = base.join("chrome-linux").join("chrome");

        if exec_path.exists() {
            return Some(exec_path.to_string_lossy().to_string());
        }
    }

    None
}

fn get_available_chrome_path() -> Option<(String, String)> {
    if let Some(path) = get_system_chrome_path() {
        return Some((path, "system".to_string()));
    }
    get_playwright_chrome_path().map(|path| (path, "playwright".to_string()))
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn profile_data_store_identifier(profile_key: &str) -> [u8; 16] {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn fnv1a64(bytes: &[u8], seed: u64) -> u64 {
        let mut hash = FNV_OFFSET_BASIS ^ seed;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    let bytes = profile_key.as_bytes();
    let h1 = fnv1a64(bytes, 0x9e3779b185ebca87);
    let h2 = fnv1a64(bytes, 0xc2b2ae3d27d4eb4f);

    let mut out = [0_u8; 16];
    out[..8].copy_from_slice(&h1.to_le_bytes());
    out[8..].copy_from_slice(&h2.to_le_bytes());
    out
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
        .map_err(|e| format!("无效的 URL: {e}"))?;

    // 获取窗口并导航
    if let Some(window) = app.get_webview_window(&panel_id) {
        // 使用 eval 来导航
        let js = format!("window.location.href = '{parsed_url}';");
        window.eval(&js).map_err(|e| format!("导航失败: {e}"))?;

        // 更新状态中的 URL
        let mut manager = state.0.write().await;
        if let Some(panel) = manager.panels.get_mut(&panel_id) {
            panel.url = url;
        }

        Ok(true)
    } else {
        Err(format!("窗口不存在: {panel_id}"))
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
            .map_err(|e| format!("设置大小失败: {e}"))?;

        // 更新状态
        let mut manager = state.0.write().await;
        if let Some(panel) = manager.panels.get_mut(&panel_id) {
            panel.width = width;
            panel.height = height;
        }

        Ok(true)
    } else {
        Err(format!("窗口不存在: {panel_id}"))
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
        window.set_focus().map_err(|e| format!("聚焦失败: {e}"))?;
        Ok(true)
    } else {
        Err(format!("窗口不存在: {panel_id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[test]
    fn profile_data_store_identifier_should_be_stable_for_same_key() {
        let left = profile_data_store_identifier("search_google");
        let right = profile_data_store_identifier("search_google");
        assert_eq!(left, right);
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[test]
    fn profile_data_store_identifier_should_differ_for_different_keys() {
        let left = profile_data_store_identifier("search_google");
        let right = profile_data_store_identifier("search_xiaohongshu");
        assert_ne!(left, right);
    }

    #[test]
    fn resolve_profile_data_dir_should_join_expected_segments() {
        let base = PathBuf::from("proxycast_data");
        let path = resolve_profile_data_dir_from_base(&base, "search_google");
        assert_eq!(
            path,
            PathBuf::from("proxycast_data/webview_profiles/search_google")
        );
    }

    #[test]
    fn sanitize_profile_key_should_replace_unsafe_chars() {
        let safe = sanitize_profile_key("search/google:zh-CN");
        assert_eq!(safe, "search_google_zh-CN");
    }

    #[test]
    fn normalize_profile_key_should_fallback_to_default() {
        let normalized = normalize_profile_key("///");
        assert_eq!(normalized, "default");
    }

    #[test]
    fn profile_remote_debugging_port_should_be_stable() {
        let left = profile_remote_debugging_port("search_google");
        let right = profile_remote_debugging_port("search_google");
        assert_eq!(left, right);
        assert!((13000..17000).contains(&left));
    }

    #[test]
    fn profile_remote_debugging_port_should_differ_for_different_keys() {
        let left = profile_remote_debugging_port("search_google");
        let right = profile_remote_debugging_port("search_xiaohongshu");
        assert_ne!(left, right);
    }

    #[test]
    fn normalize_backend_policy_should_deduplicate_and_fill_defaults() {
        let policy = BrowserBackendPolicy {
            priority: vec![BrowserBackendType::CdpDirect, BrowserBackendType::CdpDirect],
            auto_fallback: false,
        };
        let normalized = normalize_backend_policy(policy).expect("policy must normalize");
        assert_eq!(
            normalized.priority,
            vec![
                BrowserBackendType::CdpDirect,
                BrowserBackendType::AsterCompat,
                BrowserBackendType::ProxycastExtensionBridge,
            ]
        );
        assert!(!normalized.auto_fallback);
    }

    #[test]
    fn normalize_action_name_should_strip_aster_prefix() {
        let action = normalize_action_name("mcp__proxycast-browser__read_page")
            .expect("action must normalize");
        assert_eq!(action, "read_page");
    }

    #[test]
    fn build_backend_candidates_should_prefer_forced_backend() {
        let policy = BrowserBackendPolicy::default();
        let candidates = build_backend_candidates(Some(BrowserBackendType::CdpDirect), &policy);
        assert_eq!(candidates, vec![BrowserBackendType::CdpDirect]);
    }
}
