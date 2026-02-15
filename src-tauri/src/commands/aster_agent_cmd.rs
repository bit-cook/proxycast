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
use crate::workspace::WorkspaceManager;
use aster::agents::extension::{Envs, ExtensionConfig};
use aster::conversation::message::{Message, MessageContent};
use aster::permission::{
    ParameterRestriction, PermissionScope, RestrictionType, ToolPermission, ToolPermissionManager,
};
use aster::permission::{Permission, PermissionConfirmation, PrincipalType};
use aster::sandbox::{
    detect_best_sandbox, execute_in_sandbox, ResourceLimits, SandboxConfig as ProcessSandboxConfig,
};
use aster::tools::{
    BashTool, PermissionBehavior, PermissionCheckResult, Tool, ToolContext, ToolError, ToolOptions,
    ToolResult, MAX_OUTPUT_LENGTH,
};
use async_trait::async_trait;
use futures::StreamExt;
use proxycast_agent::event_converter::convert_agent_event;
use proxycast_services::mcp_service::McpService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

const DEFAULT_BASH_TIMEOUT_SECS: u64 = 300;
const MAX_BASH_TIMEOUT_SECS: u64 = 1800;

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
    /// Workspace ID（必填，用于校验会话与工作区一致性并启用本地 sandbox）
    pub workspace_id: String,
}

/// 基于 aster::sandbox 的本地 bash 强隔离工具
#[derive(Debug)]
struct WorkspaceSandboxedBashTool {
    delegate: BashTool,
    sandbox_type_name: String,
    base_sandbox_config: ProcessSandboxConfig,
}

impl WorkspaceSandboxedBashTool {
    fn new(workspace_root: &str) -> Result<Self, String> {
        let workspace_root = workspace_root.trim();
        if workspace_root.is_empty() {
            return Err("workspace 根目录为空".to_string());
        }

        let sandbox_type = detect_best_sandbox();
        let sandbox_type_name = format!("{sandbox_type:?}");
        if sandbox_type_name == "None" {
            return Err(
                "未检测到可用本地 sandbox 执行器（macOS 需 sandbox-exec，Linux 需 bwrap/firejail）"
                    .to_string(),
            );
        }

        let workspace_path = PathBuf::from(workspace_root);
        let mut read_only_paths = vec![
            PathBuf::from("/usr"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/etc"),
            PathBuf::from("/System"),
            PathBuf::from("/Library"),
            workspace_path.clone(),
        ];
        read_only_paths.sort();
        read_only_paths.dedup();

        let mut writable_paths = vec![workspace_path.clone(), PathBuf::from("/tmp")];
        if cfg!(target_os = "macos") {
            writable_paths.push(PathBuf::from("/private/tmp"));
        }
        writable_paths.sort();
        writable_paths.dedup();

        let base_sandbox_config = ProcessSandboxConfig {
            enabled: true,
            sandbox_type,
            allowed_paths: vec![workspace_path],
            denied_paths: Vec::new(),
            network_access: false,
            environment_variables: HashMap::new(),
            read_only_paths,
            writable_paths,
            allow_dev_access: false,
            allow_proc_access: false,
            allow_sys_access: false,
            env_whitelist: Vec::new(),
            tmpfs_size: "64M".to_string(),
            unshare_all: true,
            die_with_parent: true,
            new_session: true,
            docker: None,
            custom_args: Vec::new(),
            audit_logging: None,
            resource_limits: None,
        };

        Ok(Self {
            delegate: BashTool::new(),
            sandbox_type_name,
            base_sandbox_config,
        })
    }

    fn sandbox_type(&self) -> &str {
        &self.sandbox_type_name
    }

    fn build_sandbox_config(
        &self,
        context: &ToolContext,
        timeout_secs: u64,
    ) -> ProcessSandboxConfig {
        let mut config = self.base_sandbox_config.clone();

        let mut environment_variables = HashMap::new();
        environment_variables.insert("ASTER_TERMINAL".to_string(), "1".to_string());
        for (key, value) in &context.environment {
            environment_variables.insert(key.clone(), value.clone());
        }
        if let Ok(path_env) = std::env::var("PATH") {
            environment_variables
                .entry("PATH".to_string())
                .or_insert(path_env);
        }

        config.environment_variables = environment_variables;
        config.resource_limits = Some(ResourceLimits {
            max_memory: Some(1024 * 1024 * 1024),
            max_cpu: Some(70),
            max_processes: Some(32),
            max_file_size: Some(50 * 1024 * 1024),
            max_execution_time: Some(timeout_secs.saturating_mul(1000)),
            max_file_descriptors: Some(256),
        });
        config
    }

    fn quote_shell(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }

    fn build_shell_command(&self, command: &str, context: &ToolContext) -> (String, Vec<String>) {
        #[cfg(target_os = "windows")]
        {
            return (
                "powershell".to_string(),
                vec![
                    "-NoProfile".to_string(),
                    "-NonInteractive".to_string(),
                    "-Command".to_string(),
                    command.to_string(),
                ],
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            let working_dir = context.working_directory.to_string_lossy().to_string();
            let wrapped_command = format!("cd {} && {}", Self::quote_shell(&working_dir), command);
            ("sh".to_string(), vec!["-lc".to_string(), wrapped_command])
        }
    }

    fn format_output(stdout: &str, stderr: &str, exit_code: i32) -> String {
        let mut output = String::new();

        if !stdout.is_empty() {
            output.push_str(stdout);
        }

        if !stderr.is_empty() {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            if !stdout.is_empty() {
                output.push_str("--- stderr ---\n");
            }
            output.push_str(stderr);
        }

        if exit_code != 0 && output.is_empty() {
            output = format!("Command exited with code {exit_code}");
        }

        if output.len() <= MAX_OUTPUT_LENGTH {
            return output;
        }

        let bytes = output.as_bytes();
        let truncated = String::from_utf8_lossy(&bytes[..MAX_OUTPUT_LENGTH]).to_string();
        format!(
            "{}\n\n[output truncated: {} bytes total]",
            truncated,
            output.len()
        )
    }
}

#[async_trait]
impl Tool for WorkspaceSandboxedBashTool {
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn description(&self) -> &str {
        self.delegate.description()
    }

    fn input_schema(&self) -> serde_json::Value {
        self.delegate.input_schema()
    }

    fn options(&self) -> ToolOptions {
        self.delegate.options()
    }

    async fn check_permissions(
        &self,
        params: &serde_json::Value,
        context: &ToolContext,
    ) -> PermissionCheckResult {
        self.delegate.check_permissions(params, context).await
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        if context.is_cancelled() {
            return Err(ToolError::Cancelled);
        }

        let permission = self.check_permissions(&params, context).await;
        match permission.behavior {
            PermissionBehavior::Allow => {}
            PermissionBehavior::Deny => {
                let message = permission
                    .message
                    .unwrap_or_else(|| "命令被安全策略拒绝".to_string());
                return Err(ToolError::permission_denied(message));
            }
            PermissionBehavior::Ask => {
                let message = permission
                    .message
                    .unwrap_or_else(|| "命令需要人工确认".to_string());
                return Err(ToolError::permission_denied(message));
            }
        }

        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_params("Missing required parameter: command"))?;

        let background = params
            .get("background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if background {
            return Err(ToolError::invalid_params(
                "本地 sandbox 模式不支持 background=true",
            ));
        }

        let timeout_secs = params
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_BASH_TIMEOUT_SECS)
            .min(MAX_BASH_TIMEOUT_SECS);

        let sandbox_config = self.build_sandbox_config(context, timeout_secs);
        let (entry, args) = self.build_shell_command(command, context);

        let execution = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            execute_in_sandbox(&entry, &args, &sandbox_config),
        )
        .await
        .map_err(|_| ToolError::timeout(Duration::from_secs(timeout_secs)))?
        .map_err(|e| ToolError::execution_failed(format!("sandbox 执行失败: {e}")))?;

        let output = Self::format_output(&execution.stdout, &execution.stderr, execution.exit_code);
        if execution.exit_code == 0 {
            Ok(ToolResult::success(output)
                .with_metadata("exit_code", serde_json::json!(execution.exit_code))
                .with_metadata("stdout_length", serde_json::json!(execution.stdout.len()))
                .with_metadata("stderr_length", serde_json::json!(execution.stderr.len()))
                .with_metadata("sandboxed", serde_json::json!(execution.sandboxed))
                .with_metadata(
                    "sandbox_type",
                    serde_json::json!(format!("{:?}", execution.sandbox_type)),
                ))
        } else {
            Ok(ToolResult::error(output)
                .with_metadata("exit_code", serde_json::json!(execution.exit_code))
                .with_metadata("stdout_length", serde_json::json!(execution.stdout.len()))
                .with_metadata("stderr_length", serde_json::json!(execution.stderr.len()))
                .with_metadata("sandboxed", serde_json::json!(execution.sandboxed))
                .with_metadata(
                    "sandbox_type",
                    serde_json::json!(format!("{:?}", execution.sandbox_type)),
                ))
        }
    }
}

/// 为指定工作区生成本地 sandbox 权限模板
async fn apply_workspace_sandbox_permissions(
    state: &AsterAgentState,
    workspace_root: &str,
) -> Result<(), String> {
    let workspace_root = workspace_root.trim();
    if workspace_root.is_empty() {
        return Err("workspace 根目录为空".to_string());
    }

    let escaped_root = regex::escape(workspace_root);
    let workspace_path_pattern = format!(r"^({escaped_root}|\.|\./|\.\./).*$");
    let workspace_abs_path_pattern = format!(r"^({escaped_root}).*$");
    let analyze_image_path_pattern = format!(
        r"^(base64:[A-Za-z0-9+/=]+|file://({escaped_root}).*|({escaped_root}|\.|\./|\.\./).*)$"
    );
    let safe_https_url_pattern = String::from(r"^https://[^\s]+$");
    let mut permissions = vec![
        ToolPermission {
            tool: "read".to_string(),
            allowed: true,
            priority: 100,
            conditions: Vec::new(),
            parameter_restrictions: vec![ParameterRestriction {
                parameter: "path".to_string(),
                restriction_type: RestrictionType::Pattern,
                values: None,
                pattern: Some(workspace_path_pattern.clone()),
                validator: None,
                min: None,
                max: None,
                required: true,
                description: Some("read.path 必须在 workspace 内或相对路径".to_string()),
            }],
            scope: PermissionScope::Session,
            reason: Some("仅允许读取当前 workspace 内容".to_string()),
            expires_at: None,
            metadata: HashMap::new(),
        },
        ToolPermission {
            tool: "write".to_string(),
            allowed: true,
            priority: 100,
            conditions: Vec::new(),
            parameter_restrictions: vec![ParameterRestriction {
                parameter: "path".to_string(),
                restriction_type: RestrictionType::Pattern,
                values: None,
                pattern: Some(workspace_path_pattern.clone()),
                validator: None,
                min: None,
                max: None,
                required: true,
                description: Some("write.path 必须在 workspace 内或相对路径".to_string()),
            }],
            scope: PermissionScope::Session,
            reason: Some("仅允许写入当前 workspace 内容".to_string()),
            expires_at: None,
            metadata: HashMap::new(),
        },
        ToolPermission {
            tool: "edit".to_string(),
            allowed: true,
            priority: 100,
            conditions: Vec::new(),
            parameter_restrictions: vec![ParameterRestriction {
                parameter: "path".to_string(),
                restriction_type: RestrictionType::Pattern,
                values: None,
                pattern: Some(workspace_path_pattern.clone()),
                validator: None,
                min: None,
                max: None,
                required: true,
                description: Some("edit.path 必须在 workspace 内或相对路径".to_string()),
            }],
            scope: PermissionScope::Session,
            reason: Some("仅允许编辑当前 workspace 内容".to_string()),
            expires_at: None,
            metadata: HashMap::new(),
        },
        ToolPermission {
            tool: "glob".to_string(),
            allowed: true,
            priority: 100,
            conditions: Vec::new(),
            parameter_restrictions: vec![ParameterRestriction {
                parameter: "path".to_string(),
                restriction_type: RestrictionType::Pattern,
                values: None,
                pattern: Some(workspace_path_pattern.clone()),
                validator: None,
                min: None,
                max: None,
                required: false,
                description: Some("glob.path 必须在 workspace 内或相对路径".to_string()),
            }],
            scope: PermissionScope::Session,
            reason: Some("仅允许在当前 workspace 搜索文件".to_string()),
            expires_at: None,
            metadata: HashMap::new(),
        },
        ToolPermission {
            tool: "grep".to_string(),
            allowed: true,
            priority: 100,
            conditions: Vec::new(),
            parameter_restrictions: vec![ParameterRestriction {
                parameter: "path".to_string(),
                restriction_type: RestrictionType::Pattern,
                values: None,
                pattern: Some(workspace_path_pattern.clone()),
                validator: None,
                min: None,
                max: None,
                required: false,
                description: Some("grep.path 必须在 workspace 内或相对路径".to_string()),
            }],
            scope: PermissionScope::Session,
            reason: Some("仅允许在当前 workspace 搜索内容".to_string()),
            expires_at: None,
            metadata: HashMap::new(),
        },
    ];

    let allow_shell_pattern = format!(
        r"^\s*(?:cd\s+({escaped_root}|\.|\./|\.\./)|pwd|ls(?:\s+[^;&|]+)?|find\s+({escaped_root}|\.|\./|\.\./)[^;&|]*|rg\b[^;&|]*|grep\b[^;&|]*|cat\s+({escaped_root}|\.|\./|\.\./)[^;&|]*)\s*$"
    );

    permissions.push(ToolPermission {
        tool: "bash".to_string(),
        allowed: true,
        priority: 90,
        conditions: Vec::new(),
        parameter_restrictions: vec![ParameterRestriction {
            parameter: "command".to_string(),
            restriction_type: RestrictionType::Pattern,
            values: None,
            pattern: Some(allow_shell_pattern.clone()),
            validator: None,
            min: None,
            max: None,
            required: true,
            description: Some("bash.command 仅允许 workspace 内安全读操作".to_string()),
        }],
        scope: PermissionScope::Session,
        reason: Some("本地 sandbox：bash 仅允许 workspace 内安全命令".to_string()),
        expires_at: None,
        metadata: HashMap::new(),
    });

    permissions.push(ToolPermission {
        tool: "Task".to_string(),
        allowed: true,
        priority: 88,
        conditions: Vec::new(),
        parameter_restrictions: vec![ParameterRestriction {
            parameter: "command".to_string(),
            restriction_type: RestrictionType::Pattern,
            values: None,
            pattern: Some(allow_shell_pattern.clone()),
            validator: None,
            min: None,
            max: None,
            required: true,
            description: Some("Task.command 仅允许 workspace 内安全命令".to_string()),
        }],
        scope: PermissionScope::Session,
        reason: Some("本地 sandbox：Task 仅允许 workspace 内安全命令".to_string()),
        expires_at: None,
        metadata: HashMap::new(),
    });

    permissions.push(ToolPermission {
        tool: "lsp".to_string(),
        allowed: true,
        priority: 88,
        conditions: Vec::new(),
        parameter_restrictions: vec![ParameterRestriction {
            parameter: "path".to_string(),
            restriction_type: RestrictionType::Pattern,
            values: None,
            pattern: Some(workspace_path_pattern.clone()),
            validator: None,
            min: None,
            max: None,
            required: true,
            description: Some("lsp.path 必须在 workspace 内或相对路径".to_string()),
        }],
        scope: PermissionScope::Session,
        reason: Some("允许在 workspace 内使用 LSP".to_string()),
        expires_at: None,
        metadata: HashMap::new(),
    });

    permissions.push(ToolPermission {
        tool: "NotebookEdit".to_string(),
        allowed: true,
        priority: 88,
        conditions: Vec::new(),
        parameter_restrictions: vec![ParameterRestriction {
            parameter: "notebook_path".to_string(),
            restriction_type: RestrictionType::Pattern,
            values: None,
            pattern: Some(workspace_abs_path_pattern.clone()),
            validator: None,
            min: None,
            max: None,
            required: true,
            description: Some("NotebookEdit.notebook_path 必须是 workspace 内绝对路径".to_string()),
        }],
        scope: PermissionScope::Session,
        reason: Some("允许编辑 workspace 内 Notebook".to_string()),
        expires_at: None,
        metadata: HashMap::new(),
    });

    permissions.push(ToolPermission {
        tool: "analyze_image".to_string(),
        allowed: true,
        priority: 88,
        conditions: Vec::new(),
        parameter_restrictions: vec![ParameterRestriction {
            parameter: "file_path".to_string(),
            restriction_type: RestrictionType::Pattern,
            values: None,
            pattern: Some(analyze_image_path_pattern),
            validator: None,
            min: None,
            max: None,
            required: true,
            description: Some(
                "analyze_image.file_path 仅允许 base64、workspace 内绝对路径或相对路径".to_string(),
            ),
        }],
        scope: PermissionScope::Session,
        reason: Some("允许分析 workspace 内图片或 base64 数据".to_string()),
        expires_at: None,
        metadata: HashMap::new(),
    });

    permissions.push(ToolPermission {
        tool: "WebFetch".to_string(),
        allowed: true,
        priority: 88,
        conditions: Vec::new(),
        parameter_restrictions: vec![ParameterRestriction {
            parameter: "url".to_string(),
            restriction_type: RestrictionType::Pattern,
            values: None,
            pattern: Some(safe_https_url_pattern),
            validator: None,
            min: None,
            max: None,
            required: true,
            description: Some("WebFetch.url 仅允许 https 且禁止内网/本机地址".to_string()),
        }],
        scope: PermissionScope::Session,
        reason: Some("允许安全的 WebFetch 请求".to_string()),
        expires_at: None,
        metadata: HashMap::new(),
    });

    for tool_name in [
        "Skill",
        "TaskOutput",
        "KillShell",
        "TodoWrite",
        "EnterPlanMode",
        "ExitPlanMode",
        "WebSearch",
        "ask",
        "three_stage_workflow",
    ] {
        permissions.push(ToolPermission {
            tool: tool_name.to_string(),
            allowed: true,
            priority: 88,
            conditions: Vec::new(),
            parameter_restrictions: Vec::new(),
            scope: PermissionScope::Session,
            reason: Some(format!("允许默认工具: {tool_name}")),
            expires_at: None,
            metadata: HashMap::new(),
        });
    }

    permissions.push(ToolPermission {
        tool: "*".to_string(),
        allowed: false,
        priority: 10,
        conditions: Vec::new(),
        parameter_restrictions: Vec::new(),
        scope: PermissionScope::Session,
        reason: Some("本地 sandbox：未显式授权的工具默认拒绝".to_string()),
        expires_at: None,
        metadata: HashMap::new(),
    });

    let agent_arc = state.get_agent_arc();
    let guard = agent_arc.read().await;
    let agent = guard
        .as_ref()
        .ok_or_else(|| "Agent not initialized".to_string())?;
    let registry_arc = agent.tool_registry().clone();
    drop(guard);

    let mut registry = registry_arc.write().await;
    let mut permission_manager = ToolPermissionManager::new(None);
    if let Some(existing_manager) = registry.permission_manager() {
        for permission in existing_manager.get_permissions(None) {
            let scope = permission.scope;
            permission_manager.add_permission(permission, scope);
        }
    }

    for permission in permissions {
        permission_manager.add_permission(permission, PermissionScope::Session);
    }
    registry.set_permission_manager(Arc::new(permission_manager));

    let workspace_bash_tool = WorkspaceSandboxedBashTool::new(workspace_root)?;
    let sandbox_type = workspace_bash_tool.sandbox_type().to_string();
    registry.register(Box::new(workspace_bash_tool));

    tracing::info!(
        "[AsterAgent] 已应用 workspace 本地 sandbox: root={}, type={}",
        workspace_root,
        sandbox_type
    );

    Ok(())
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

    let workspace_id = request.workspace_id.trim().to_string();
    if workspace_id.is_empty() {
        return Err("workspace_id 必填，请先选择项目工作区".to_string());
    }

    let manager = WorkspaceManager::new(db.inner().clone());
    let workspace = manager
        .get(&workspace_id)
        .map_err(|e| format!("读取 workspace 失败: {e}"))?
        .ok_or_else(|| format!("Workspace 不存在: {workspace_id}"))?;
    let workspace_root = workspace.root_path.to_string_lossy().to_string();

    {
        let db_conn = db.lock().map_err(|e| format!("获取数据库连接失败: {e}"))?;
        if let Some(session) = AgentDao::get_session(&db_conn, session_id)
            .map_err(|e| format!("读取 session 失败: {e}"))?
        {
            let session_dir = session.working_dir.unwrap_or_default();
            if !session_dir.is_empty() && session_dir != workspace_root {
                tracing::warn!(
                    "[AsterAgent] workspace mismatch: session_id={}, workspace_id={}, session_dir={}, workspace_root={}",
                    session_id,
                    workspace_id,
                    session_dir,
                    workspace_root
                );
                return Err(format!(
                    "workspace_mismatch|会话工作目录与 workspace 不匹配: session={session_dir}, workspace={workspace_root}"
                ));
            }
        }
    }

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

    apply_workspace_sandbox_permissions(&state, &workspace_root)
        .await
        .map_err(|e| format!("注入本地 sandbox 失败: {e}"))?;

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
    working_dir: Option<String>,
    workspace_id: String,
    name: Option<String>,
) -> Result<String, String> {
    tracing::info!("[AsterAgent] 创建会话: name={:?}", name);

    let workspace_id = workspace_id.trim().to_string();
    if workspace_id.is_empty() {
        return Err("workspace_id 必填，请先选择项目工作区".to_string());
    }

    AsterAgentWrapper::create_session_sync(&db, name, working_dir, workspace_id)
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

/// 重命名会话
#[tauri::command]
pub async fn aster_session_rename(
    db: State<'_, DbConnection>,
    session_id: String,
    name: String,
) -> Result<(), String> {
    tracing::info!("[AsterAgent] 重命名会话: {}", session_id);
    AsterAgentWrapper::rename_session_sync(&db, &session_id, &name)
}

/// 删除会话
#[tauri::command]
pub async fn aster_session_delete(
    db: State<'_, DbConnection>,
    session_id: String,
) -> Result<(), String> {
    tracing::info!("[AsterAgent] 删除会话: {}", session_id);
    AsterAgentWrapper::delete_session_sync(&db, &session_id)
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
    state: State<'_, AsterAgentState>,
    request: ConfirmRequest,
) -> Result<(), String> {
    tracing::info!(
        "[AsterAgent] 确认请求: id={}, confirmed={}",
        request.request_id,
        request.confirmed
    );

    let permission = if request.confirmed {
        Permission::AllowOnce
    } else {
        Permission::DenyOnce
    };

    let confirmation = PermissionConfirmation {
        principal_type: PrincipalType::Tool,
        permission,
    };

    let agent_arc = state.get_agent_arc();
    let guard = agent_arc.read().await;
    let agent = guard.as_ref().ok_or("Agent not initialized")?;
    agent
        .handle_confirmation(request.request_id.clone(), confirmation)
        .await;

    Ok(())
}

/// Elicitation 回填请求
#[derive(Debug, Deserialize)]
pub struct SubmitElicitationResponseRequest {
    pub request_id: String,
    pub user_data: serde_json::Value,
}

fn validate_elicitation_submission(session_id: &str, request_id: &str) -> Result<String, String> {
    let trimmed_session_id = session_id.trim().to_string();
    if trimmed_session_id.is_empty() {
        return Err("session_id 不能为空".to_string());
    }
    if request_id.trim().is_empty() {
        return Err("request_id 不能为空".to_string());
    }
    Ok(trimmed_session_id)
}

/// 提交 elicitation 回答（用于 ask/lsp 等需要用户输入的流程）
#[tauri::command]
pub async fn aster_agent_submit_elicitation_response(
    state: State<'_, AsterAgentState>,
    session_id: String,
    request: SubmitElicitationResponseRequest,
) -> Result<(), String> {
    let session_id = validate_elicitation_submission(&session_id, &request.request_id)?;

    tracing::info!(
        "[AsterAgent] 提交 elicitation 响应: session={}, request_id={}",
        session_id,
        request.request_id
    );

    let message =
        Message::user().with_content(MessageContent::action_required_elicitation_response(
            request.request_id.clone(),
            request.user_data,
        ));

    let session_config = SessionConfigBuilder::new(&session_id).build();

    let agent_arc = state.get_agent_arc();
    let guard = agent_arc.read().await;
    let agent = guard.as_ref().ok_or("Agent not initialized")?;

    let _ = agent
        .reply(message, session_config, None)
        .await
        .map_err(|e| format!("提交 elicitation 响应失败: {e}"))?;

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
            "event_name": "agent_stream",
            "workspace_id": "workspace-test"
        }"#;

        let request: AsterChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message, "Hello");
        assert_eq!(request.session_id, "test-session");
        assert_eq!(request.event_name, "agent_stream");
        assert_eq!(request.workspace_id, "workspace-test");
    }

    #[test]
    fn test_validate_elicitation_submission_rejects_empty_session_id() {
        let result = validate_elicitation_submission("   ", "req-1");
        assert_eq!(result, Err("session_id 不能为空".to_string()));
    }

    #[test]
    fn test_validate_elicitation_submission_rejects_empty_request_id() {
        let result = validate_elicitation_submission("session-1", "   ");
        assert_eq!(result, Err("request_id 不能为空".to_string()));
    }

    #[test]
    fn test_validate_elicitation_submission_trims_session_id() {
        let result = validate_elicitation_submission("  session-1  ", "req-1");
        assert_eq!(result, Ok("session-1".to_string()));
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
