//! Tauri Setup Hook
//!
//! 包含应用启动时的初始化逻辑。

use std::sync::Arc;
use tauri::{App, Manager};

// use crate::agent::tools::{set_term_scrollback_tool_app_handle, set_terminal_tool_app_handle};
use crate::agent::NativeAgentState;
use crate::commands::oauth_plugin_cmd::OAuthPluginManagerState;
use crate::database;
use crate::flow_monitor::FlowInterceptor;
use crate::services::provider_pool_service::ProviderPoolService;
use crate::services::token_cache_service::TokenCacheService;
use crate::telemetry;
use crate::tray::{TrayIconStatus, TrayManager, TrayStateSnapshot};

use super::types::{AppState, LogState, TrayManagerState};

/// Tauri setup hook
///
/// 在应用启动时执行初始化逻辑
pub fn setup_app(
    app: &mut App,
    state: AppState,
    logs: LogState,
    db: database::DbConnection,
    pool_service: Arc<ProviderPoolService>,
    token_cache: Arc<TokenCacheService>,
    shared_stats: Arc<parking_lot::RwLock<telemetry::StatsAggregator>>,
    shared_tokens: Arc<parking_lot::RwLock<telemetry::TokenTracker>>,
    shared_logger: Arc<telemetry::RequestLogger>,
    flow_monitor: Arc<crate::flow_monitor::FlowMonitor>,
    flow_interceptor: Arc<FlowInterceptor>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 初始化托盘管理器
    match TrayManager::new(app.handle()) {
        Ok(tray_manager) => {
            tracing::info!("[启动] 托盘管理器初始化成功");
            let tray_state: TrayManagerState<tauri::Wry> =
                TrayManagerState(Arc::new(tokio::sync::RwLock::new(Some(tray_manager))));
            app.manage(tray_state);
        }
        Err(e) => {
            tracing::error!("[启动] 托盘管理器初始化失败: {}", e);
            let tray_state: TrayManagerState<tauri::Wry> =
                TrayManagerState(Arc::new(tokio::sync::RwLock::new(None)));
            app.manage(tray_state);
        }
    }

    // 初始化 NativeAgentState
    let native_agent_state = NativeAgentState::new();
    app.manage(native_agent_state);

    // TODO: 重新实现 TerminalTool 和 TermScrollbackTool 的 AppHandle 设置
    // 当前暂时注释掉，等待适配 aster-rust 工具系统
    // set_terminal_tool_app_handle(app.handle().clone());
    // tracing::info!("[启动] TerminalTool AppHandle 已设置");

    // set_term_scrollback_tool_app_handle(app.handle().clone());
    // tracing::info!("[启动] TermScrollbackTool AppHandle 已设置");

    // 初始化 OAuth Plugin Manager State
    let oauth_plugin_manager_state = OAuthPluginManagerState::with_defaults();
    app.manage(oauth_plugin_manager_state);

    // 初始化默认 skill repos
    {
        let conn = db.lock().expect("Failed to lock database");
        database::dao::skills::SkillDao::init_default_skill_repos(&conn)
            .expect("Failed to initialize default skill repos");
    }

    // 自动启动服务器
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        start_server_async(
            state,
            logs,
            db,
            pool_service,
            token_cache,
            shared_stats,
            shared_tokens,
            shared_logger,
            flow_monitor,
            flow_interceptor,
            app_handle,
        )
        .await;
    });

    Ok(())
}

/// 异步启动服务器
async fn start_server_async(
    state: AppState,
    logs: LogState,
    db: database::DbConnection,
    pool_service: Arc<ProviderPoolService>,
    token_cache: Arc<TokenCacheService>,
    shared_stats: Arc<parking_lot::RwLock<telemetry::StatsAggregator>>,
    shared_tokens: Arc<parking_lot::RwLock<telemetry::TokenTracker>>,
    shared_logger: Arc<telemetry::RequestLogger>,
    shared_flow_monitor: Arc<crate::flow_monitor::FlowMonitor>,
    flow_interceptor: Arc<FlowInterceptor>,
    app_handle: tauri::AppHandle,
) {
    // 先加载凭证池中的凭证
    {
        logs.write().await.add("info", "[启动] 正在加载凭证池...");

        match pool_service.get_overview(&db) {
            Ok(overview) => {
                let mut loaded_types = Vec::new();
                let mut total_credentials = 0;

                for provider_overview in overview {
                    let count = provider_overview.stats.total_count;
                    if count > 0 {
                        total_credentials += count;
                        let provider_name = match provider_overview.provider_type.as_str() {
                            "kiro" => "Kiro",
                            "gemini" => "Gemini",
                            "qwen" => "通义千问",
                            "antigravity" => "Antigravity",
                            "openai" => "OpenAI",
                            "claude" => "Claude",
                            "codex" => "Codex",
                            "claude_oauth" => "Claude OAuth",
                            "iflow" => "iFlow",
                            _ => &provider_overview.provider_type,
                        };
                        loaded_types.push(format!("{} ({} 个)", provider_name, count));
                    }
                }

                if loaded_types.is_empty() {
                    logs.write().await.add("warn", "[启动] 未找到任何可用凭证");
                } else {
                    let message = format!(
                        "[启动] 凭证已加载: {} (共 {} 个)",
                        loaded_types.join(", "),
                        total_credentials
                    );
                    logs.write().await.add("info", &message);
                }
            }
            Err(e) => {
                logs.write()
                    .await
                    .add("warn", &format!("[启动] 获取凭证池信息失败: {}", e));
            }
        }

        // 兼容性：仍然尝试加载旧的 Kiro 凭证（如果存在）
        let mut s = state.write().await;
        if let Err(e) = s.kiro_provider.load_credentials().await {
            logs.write()
                .await
                .add("debug", &format!("[启动] 旧版 Kiro 凭证加载失败: {e}"));
        }
    }

    // 启动服务器
    let server_started;
    let server_address;
    {
        let mut s = state.write().await;
        logs.write()
            .await
            .add("info", "[启动] 正在自动启动服务器...");
        match s
            .start_with_telemetry_and_flow_monitor(
                logs.clone(),
                pool_service,
                token_cache,
                Some(db),
                Some(shared_stats),
                Some(shared_tokens),
                Some(shared_logger),
                Some(shared_flow_monitor),
                Some(flow_interceptor),
            )
            .await
        {
            Ok(_) => {
                // 获取服务器实际使用的地址（可能已经自动切换到有效的 IP）
                let status = s.status();
                let host = status.host;
                let port = status.port;
                logs.write()
                    .await
                    .add("info", &format!("[启动] 服务器已启动: {host}:{port}"));
                server_started = true;
                server_address = format!("{}:{}", host, port);
            }
            Err(e) => {
                logs.write()
                    .await
                    .add("error", &format!("[启动] 服务器启动失败: {e}"));
                server_started = false;
                server_address = String::new();
            }
        }
    }

    // 更新托盘状态
    if let Some(tray_state) = app_handle.try_state::<TrayManagerState<tauri::Wry>>() {
        let tray_guard = tray_state.0.read().await;
        if let Some(tray_manager) = tray_guard.as_ref() {
            let icon_status = if server_started {
                TrayIconStatus::Running
            } else {
                TrayIconStatus::Stopped
            };

            let snapshot = TrayStateSnapshot {
                icon_status,
                server_running: server_started,
                server_address,
                available_credentials: 0,
                total_credentials: 0,
                today_requests: 0,
                auto_start_enabled: false,
            };

            if let Err(e) = tray_manager.update_state(snapshot).await {
                tracing::error!("[启动] 更新托盘状态失败: {}", e);
            } else {
                tracing::info!("[启动] 托盘状态已更新");
            }
        }
    }
}
