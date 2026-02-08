//! Aster Agent 命令模块
//!
//! 提供基于 Aster 框架的 Tauri 命令
//! 这是新的对话系统实现，与 native_agent_cmd.rs 并行存在
//! 支持从 ProxyCast 凭证池自动选择凭证

use crate::agent::aster_state::{ProviderConfig, SessionConfigBuilder};
use crate::agent::{
    AsterAgentState, AsterAgentWrapper, SessionDetail, SessionInfo, TauriAgentEvent,
};
use crate::database::dao::agent::AgentDao;
use crate::database::DbConnection;
use crate::mcp::{McpManagerState, McpServerConfig};
use aster::agents::extension::{Envs, ExtensionConfig};
use aster::conversation::message::Message;
use futures::StreamExt;
use proxycast_agent::event_converter::convert_agent_event;
use proxycast_services::mcp_service::McpService;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

/// Aster Agent 状态信息
#[derive(Debug, Serialize)]
pub struct AsterAgentStatus {
    pub initialized: bool,
    pub provider_configured: bool,
    pub provider_name: Option<String>,
    pub model_name: Option<String>,
    /// 凭证 UUID（来自凭证池）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_uuid: Option<String>,
}

/// Provider 配置请求
#[derive(Debug, Deserialize)]
pub struct ConfigureProviderRequest {
    pub provider_name: String,
    pub model_name: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// 从凭证池配置 Provider 的请求
#[derive(Debug, Deserialize)]
pub struct ConfigureFromPoolRequest {
    /// Provider 类型 (openai, anthropic, kiro, gemini 等)
    pub provider_type: String,
    /// 模型名称
    pub model_name: String,
}

/// 初始化 Aster Agent
#[tauri::command]
pub async fn aster_agent_init(
    state: State<'_, AsterAgentState>,
    db: State<'_, DbConnection>,
) -> Result<AsterAgentStatus, String> {
    tracing::info!("[AsterAgent] 初始化 Agent");

    state.init_agent_with_db(&db).await?;

    let provider_config = state.get_provider_config().await;

    tracing::info!("[AsterAgent] Agent 初始化成功");

    Ok(AsterAgentStatus {
        initialized: true,
        provider_configured: provider_config.is_some(),
        provider_name: provider_config.as_ref().map(|c| c.provider_name.clone()),
        model_name: provider_config.as_ref().map(|c| c.model_name.clone()),
        credential_uuid: provider_config.and_then(|c| c.credential_uuid),
    })
}

/// 配置 Aster Agent 的 Provider
#[tauri::command]
pub async fn aster_agent_configure_provider(
    state: State<'_, AsterAgentState>,
    db: State<'_, DbConnection>,
    request: ConfigureProviderRequest,
    session_id: String,
) -> Result<AsterAgentStatus, String> {
    tracing::info!(
        "[AsterAgent] 配置 Provider: {} / {}",
        request.provider_name,
        request.model_name
    );

    let config = ProviderConfig {
        provider_name: request.provider_name,
        model_name: request.model_name,
        api_key: request.api_key,
        base_url: request.base_url,
        credential_uuid: None,
    };

    state
        .configure_provider(config.clone(), &session_id, &db)
        .await?;

    Ok(AsterAgentStatus {
        initialized: true,
        provider_configured: true,
        provider_name: Some(config.provider_name),
        model_name: Some(config.model_name),
        credential_uuid: None,
    })
}

/// 从凭证池配置 Aster Agent 的 Provider
///
/// 自动从 ProxyCast 凭证池选择可用凭证并配置 Aster Provider
#[tauri::command]
pub async fn aster_agent_configure_from_pool(
    state: State<'_, AsterAgentState>,
    db: State<'_, DbConnection>,
    request: ConfigureFromPoolRequest,
    session_id: String,
) -> Result<AsterAgentStatus, String> {
    tracing::info!(
        "[AsterAgent] 从凭证池配置 Provider: {} / {}",
        request.provider_type,
        request.model_name
    );

    let aster_config = state
        .configure_provider_from_pool(
            &db,
            &request.provider_type,
            &request.model_name,
            &session_id,
        )
        .await?;

    Ok(AsterAgentStatus {
        initialized: true,
        provider_configured: true,
        provider_name: Some(aster_config.provider_name),
        model_name: Some(aster_config.model_name),
        credential_uuid: Some(aster_config.credential_uuid),
    })
}

/// 获取 Aster Agent 状态
#[tauri::command]
pub async fn aster_agent_status(
    state: State<'_, AsterAgentState>,
) -> Result<AsterAgentStatus, String> {
    let provider_config = state.get_provider_config().await;
    Ok(AsterAgentStatus {
        initialized: state.is_initialized().await,
        provider_configured: provider_config.is_some(),
        provider_name: provider_config.as_ref().map(|c| c.provider_name.clone()),
        model_name: provider_config.as_ref().map(|c| c.model_name.clone()),
        credential_uuid: provider_config.and_then(|c| c.credential_uuid),
    })
}

/// 重置 Aster Agent
///
/// 清除当前 Provider 配置，下次对话时会重新从凭证池选择凭证。
/// 用于切换凭证后无需重启应用即可生效。
#[tauri::command]
pub async fn aster_agent_reset(
    state: State<'_, AsterAgentState>,
) -> Result<AsterAgentStatus, String> {
    tracing::info!("[AsterAgent] 重置 Agent Provider 配置");

    // 清除当前 Provider 配置
    state.clear_provider_config().await;

    Ok(AsterAgentStatus {
        initialized: state.is_initialized().await,
        provider_configured: false,
        provider_name: None,
        model_name: None,
        credential_uuid: None,
    })
}

/// 发送消息请求参数
#[derive(Debug, Deserialize)]
pub struct AsterChatRequest {
    pub message: String,
    pub session_id: String,
    pub event_name: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub images: Option<Vec<ImageInput>>,
    /// Provider 配置（可选，如果未配置则使用当前配置）
    #[serde(default)]
    pub provider_config: Option<ConfigureProviderRequest>,
    /// 项目 ID（可选，用于注入项目上下文到 System Prompt）
    #[serde(default)]
    pub project_id: Option<String>,
}

/// 图片输入
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ImageInput {
    pub data: String,
    pub media_type: String,
}

/// 发送消息并获取流式响应
#[tauri::command]
pub async fn aster_agent_chat_stream(
    app: AppHandle,
    state: State<'_, AsterAgentState>,
    db: State<'_, DbConnection>,
    mcp_manager: State<'_, McpManagerState>,
    request: AsterChatRequest,
) -> Result<(), String> {
    tracing::info!(
        "[AsterAgent] 发送流式消息: session={}, event={}",
        request.session_id,
        request.event_name
    );

    // 确保 Agent 已初始化（使用带数据库的版本，注入 SessionStore）
    let is_init = state.is_initialized().await;
    tracing::warn!("[AsterAgent] Agent 初始化状态: {}", is_init);
    if !is_init {
        tracing::warn!("[AsterAgent] Agent 未初始化，开始初始化...");
        state.init_agent_with_db(&db).await?;
        tracing::warn!("[AsterAgent] Agent 初始化完成");
    } else {
        tracing::warn!("[AsterAgent] Agent 已初始化，检查 session_store...");
        // 检查 session_store 是否存在
        let agent_arc = state.get_agent_arc();
        let guard = agent_arc.read().await;
        if let Some(agent) = guard.as_ref() {
            let has_store = agent.session_store().is_some();
            tracing::warn!("[AsterAgent] session_store 存在: {}", has_store);
        }
    }

    // 直接使用前端传递的 session_id
    // ProxyCastSessionStore 会在 add_message 时自动创建不存在的 session
    // 同时 get_session 也会自动创建不存在的 session
    let session_id = &request.session_id;

    // 启动并注入 MCP extensions 到 Aster Agent
    let (_start_ok, start_fail) = ensure_proxycast_mcp_servers_running(&db, &mcp_manager).await;
    if start_fail > 0 {
        tracing::warn!(
            "[AsterAgent] 部分 MCP server 自动启动失败 ({} 失败)，后续可用工具可能不完整",
            start_fail
        );
    }

    let (_mcp_ok, mcp_fail) = inject_mcp_extensions(&state, &mcp_manager).await;
    if mcp_fail > 0 {
        tracing::warn!(
            "[AsterAgent] 部分 MCP extension 注入失败 ({} 失败)，Agent 可能无法使用某些 MCP 工具",
            mcp_fail
        );
    }

    // 构建 system_prompt：优先使用项目上下文，其次使用 session 的 system_prompt
    let system_prompt = {
        let db_conn = db.lock().map_err(|e| format!("获取数据库连接失败: {e}"))?;

        // 1. 如果提供了 project_id，构建项目上下文
        let project_prompt = if let Some(ref project_id) = request.project_id {
            match AsterAgentState::build_project_system_prompt(&db, project_id) {
                Ok(prompt) => {
                    tracing::info!(
                        "[AsterAgent] 已加载项目上下文: project_id={}, prompt_len={}",
                        project_id,
                        prompt.len()
                    );
                    Some(prompt)
                }
                Err(e) => {
                    tracing::warn!(
                        "[AsterAgent] 加载项目上下文失败: {}, 继续使用 session prompt",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        // 2. 如果没有项目上下文，尝试从 session 读取
        if project_prompt.is_some() {
            project_prompt
        } else {
            match AgentDao::get_session(&db_conn, session_id) {
                Ok(Some(session)) => {
                    tracing::debug!(
                        "[AsterAgent] 找到 session，system_prompt: {:?}",
                        session.system_prompt.as_ref().map(|s| s.len())
                    );
                    session.system_prompt
                }
                Ok(None) => {
                    tracing::debug!(
                        "[AsterAgent] ProxyCast 数据库中未找到 session: {}",
                        session_id
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!(
                        "[AsterAgent] 读取 session 失败: {}, 继续使用空 system_prompt",
                        e
                    );
                    None
                }
            }
        }
    };

    // 如果提供了 Provider 配置，则配置 Provider
    if let Some(provider_config) = &request.provider_config {
        tracing::info!(
            "[AsterAgent] 收到 provider_config: provider_name={}, model_name={}, has_api_key={}, base_url={:?}",
            provider_config.provider_name,
            provider_config.model_name,
            provider_config.api_key.is_some(),
            provider_config.base_url
        );
        let config = ProviderConfig {
            provider_name: provider_config.provider_name.clone(),
            model_name: provider_config.model_name.clone(),
            api_key: provider_config.api_key.clone(),
            base_url: provider_config.base_url.clone(),
            credential_uuid: None,
        };
        // 如果前端提供了 api_key，直接使用；否则从凭证池选择凭证
        if provider_config.api_key.is_some() {
            state.configure_provider(config, session_id, &db).await?;
        } else {
            // 没有 api_key，使用凭证池（provider_name 作为 provider_type）
            state
                .configure_provider_from_pool(
                    &db,
                    &provider_config.provider_name,
                    &provider_config.model_name,
                    session_id,
                )
                .await?;
        }
    }

    // 检查 Provider 是否已配置
    if !state.is_provider_configured().await {
        return Err("Provider 未配置，请先调用 aster_agent_configure_provider".to_string());
    }

    // 创建取消令牌
    let cancel_token = state.create_cancel_token(session_id).await;

    // 创建用户消息
    let user_message = Message::user().with_text(&request.message);

    // 创建会话配置
    let mut session_config_builder = SessionConfigBuilder::new(session_id);
    if let Some(prompt) = system_prompt {
        session_config_builder = session_config_builder.system_prompt(prompt);
    }
    let session_config = session_config_builder.build();

    // 获取 Agent Arc 并保持 guard 在整个流处理期间存活
    let agent_arc = state.get_agent_arc();
    let guard = agent_arc.read().await;
    let agent = guard.as_ref().ok_or("Agent not initialized")?;

    // 获取事件流
    let stream_result = agent
        .reply(user_message, session_config, Some(cancel_token.clone()))
        .await;

    match stream_result {
        Ok(mut stream) => {
            // 处理事件流
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(agent_event) => {
                        // 转换 Aster 事件为 Tauri 事件
                        let tauri_events = convert_agent_event(agent_event);

                        // 发送每个事件到前端
                        for tauri_event in tauri_events {
                            if let Err(e) = app.emit(&request.event_name, &tauri_event) {
                                tracing::error!("[AsterAgent] 发送事件失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        // 发送错误事件
                        let error_event = TauriAgentEvent::Error {
                            message: format!("Stream error: {e}"),
                        };
                        if let Err(emit_err) = app.emit(&request.event_name, &error_event) {
                            tracing::error!("[AsterAgent] 发送错误事件失败: {}", emit_err);
                        }
                    }
                }
            }

            // 发送完成事件
            let done_event = TauriAgentEvent::FinalDone { usage: None };
            if let Err(e) = app.emit(&request.event_name, &done_event) {
                tracing::error!("[AsterAgent] 发送完成事件失败: {}", e);
            }
        }
        Err(e) => {
            // 发送错误事件
            let error_event = TauriAgentEvent::Error {
                message: format!("Agent error: {e}"),
            };
            if let Err(emit_err) = app.emit(&request.event_name, &error_event) {
                tracing::error!("[AsterAgent] 发送错误事件失败: {}", emit_err);
            }
            return Err(format!("Agent error: {e}"));
        }
    }

    // guard 会在函数结束时自动释放（stream_result 先释放）

    // 清理取消令牌
    state.remove_cancel_token(session_id).await;

    Ok(())
}

/// 停止当前会话
#[tauri::command]
pub async fn aster_agent_stop(
    state: State<'_, AsterAgentState>,
    session_id: String,
) -> Result<bool, String> {
    tracing::info!("[AsterAgent] 停止会话: {}", session_id);
    Ok(state.cancel_session(&session_id).await)
}

/// 创建新会话
#[tauri::command]
pub async fn aster_session_create(
    db: State<'_, DbConnection>,
    name: Option<String>,
) -> Result<String, String> {
    tracing::info!("[AsterAgent] 创建会话: name={:?}", name);
    AsterAgentWrapper::create_session_sync(&db, name)
}

/// 列出所有会话
#[tauri::command]
pub async fn aster_session_list(db: State<'_, DbConnection>) -> Result<Vec<SessionInfo>, String> {
    tracing::info!("[AsterAgent] 列出会话");
    AsterAgentWrapper::list_sessions_sync(&db)
}

/// 获取会话详情
#[tauri::command]
pub async fn aster_session_get(
    db: State<'_, DbConnection>,
    session_id: String,
) -> Result<SessionDetail, String> {
    tracing::info!("[AsterAgent] 获取会话: {}", session_id);
    AsterAgentWrapper::get_session_sync(&db, &session_id)
}

/// 确认权限请求
#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    pub request_id: String,
    pub confirmed: bool,
    #[allow(dead_code)]
    pub response: Option<String>,
}

/// 确认权限请求（用于工具调用确认等）
#[tauri::command]
pub async fn aster_agent_confirm(
    _state: State<'_, AsterAgentState>,
    request: ConfirmRequest,
) -> Result<(), String> {
    tracing::info!(
        "[AsterAgent] 确认请求: id={}, confirmed={}",
        request.request_id,
        request.confirmed
    );

    // TODO: 实现权限确认逻辑
    // 这需要 Aster 框架支持 confirmation_tx 通道
    // 目前先返回成功

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aster_chat_request_deserialize() {
        let json = r#"{
            "message": "Hello",
            "session_id": "test-session",
            "event_name": "agent_stream"
        }"#;

        let request: AsterChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message, "Hello");
        assert_eq!(request.session_id, "test-session");
        assert_eq!(request.event_name, "agent_stream");
    }
}

/// 将 ProxyCast 已运行的 MCP servers 注入到 Aster Agent 作为 extensions
///
/// 获取 McpClientManager 中所有已运行的 server 配置，
/// 转换为 Aster 的 ExtensionConfig::Stdio 并注册到 Agent。
///
/// 关键：将当前进程的 PATH 等环境变量合并到 MCP server 的 env 中，
/// 确保 Aster 启动的子进程能找到 npx/uvx 等命令。
///
/// 返回 (成功数, 失败数)
async fn inject_mcp_extensions(
    state: &AsterAgentState,
    mcp_manager: &McpManagerState,
) -> (usize, usize) {
    let manager = mcp_manager.lock().await;
    let running_servers = manager.get_running_servers().await;

    if running_servers.is_empty() {
        tracing::debug!("[AsterAgent] 没有运行中的 MCP servers，跳过注入");
        return (0, 0);
    }

    let agent_arc = state.get_agent_arc();
    let guard = agent_arc.read().await;
    let agent = match guard.as_ref() {
        Some(a) => a,
        None => {
            tracing::warn!("[AsterAgent] Agent 未初始化，无法注入 MCP extensions");
            return (0, running_servers.len());
        }
    };

    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    for server_name in &running_servers {
        // 检查是否已注册（避免重复注册）
        let ext_configs = agent.get_extension_configs().await;
        if ext_configs.iter().any(|c| c.name() == *server_name) {
            tracing::debug!("[AsterAgent] MCP extension '{}' 已注册，跳过", server_name);
            success_count += 1;
            continue;
        }

        if let Some(config) = manager.get_client_config(server_name).await {
            // 合并当前进程的关键环境变量到 MCP server 的 env 中
            // 确保子进程能找到 npx/uvx/node 等命令
            let mut merged_env = config.env.clone();
            for key in &["PATH", "HOME", "USER", "SHELL", "NODE_PATH", "NVM_DIR"] {
                if !merged_env.contains_key(*key) {
                    if let Ok(val) = std::env::var(key) {
                        merged_env.insert(key.to_string(), val);
                    }
                }
            }

            tracing::info!(
                "[AsterAgent] 注入 MCP extension '{}': cmd='{}', args={:?}, env_keys={:?}",
                server_name,
                config.command,
                config.args,
                merged_env.keys().collect::<Vec<_>>()
            );

            // 增加超时时间：npx 首次下载可能需要较长时间
            let timeout = std::cmp::max(config.timeout, 60);

            let extension = ExtensionConfig::Stdio {
                name: server_name.clone(),
                description: format!("MCP Server: {server_name}"),
                cmd: config.command.clone(),
                args: config.args.clone(),
                envs: Envs::new(merged_env),
                env_keys: vec![],
                timeout: Some(timeout),
                bundled: Some(false),
                available_tools: vec![],
            };

            match agent.add_extension(extension).await {
                Ok(_) => {
                    tracing::info!("[AsterAgent] 成功注入 MCP extension: {}", server_name);
                    success_count += 1;
                }
                Err(e) => {
                    tracing::error!(
                        "[AsterAgent] 注入 MCP extension '{}' 失败: {}。\
                        cmd='{}', args={:?}。请检查命令是否在 PATH 中可用。",
                        server_name,
                        e,
                        config.command,
                        config.args
                    );
                    fail_count += 1;
                }
            }
        } else {
            tracing::warn!("[AsterAgent] 无法获取 MCP server '{}' 的配置", server_name);
            fail_count += 1;
        }
    }

    if fail_count > 0 {
        tracing::warn!(
            "[AsterAgent] MCP 注入结果: {} 成功, {} 失败",
            success_count,
            fail_count
        );
    } else {
        tracing::info!(
            "[AsterAgent] MCP 注入完成: {} 个 extension 全部成功",
            success_count
        );
    }

    (success_count, fail_count)
}

/// 确保 ProxyCast 可用的 MCP servers 已启动
///
/// 启动启用了 `enabled_proxycast` 的服务器。
async fn ensure_proxycast_mcp_servers_running(
    db: &DbConnection,
    mcp_manager: &McpManagerState,
) -> (usize, usize) {
    let servers = match McpService::get_all(db) {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!("[AsterAgent] 读取 MCP 配置失败，跳过自动启动: {}", e);
            return (0, 0);
        }
    };

    if servers.is_empty() {
        return (0, 0);
    }

    let candidates: Vec<&crate::models::mcp_model::McpServer> =
        servers.iter().filter(|s| s.enabled_proxycast).collect();

    if candidates.is_empty() {
        return (0, 0);
    }

    let manager = mcp_manager.lock().await;
    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    for server in candidates {
        if manager.is_server_running(&server.name).await {
            continue;
        }

        let parsed = server.parse_config();
        let config = McpServerConfig {
            command: parsed.command,
            args: parsed.args,
            env: parsed.env,
            cwd: parsed.cwd,
            timeout: parsed.timeout,
        };

        match manager.start_server(&server.name, &config).await {
            Ok(_) => {
                tracing::info!("[AsterAgent] MCP server 已自动启动: {}", server.name);
                success_count += 1;
            }
            Err(e) => {
                tracing::error!(
                    "[AsterAgent] MCP server 自动启动失败: {} => {}",
                    server.name,
                    e
                );
                fail_count += 1;
            }
        }
    }

    (success_count, fail_count)
}
