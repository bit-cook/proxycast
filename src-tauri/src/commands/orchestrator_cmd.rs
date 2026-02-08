//! 模型编排器 Tauri 命令
//!
//! 提供前端访问模型编排器的接口。

use crate::database::dao::provider_pool::ProviderPoolDao;
use crate::database::DbConnection;
use proxycast_core::orchestrator::{
    get_global_orchestrator, init_global_orchestrator, AvailableModel, CredentialInfo,
    OrchestratorConfig, PoolStats, ProviderType, SelectionContext, SelectionResult, ServiceTier,
    StrategyInfo, TaskHint,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock;

/// 编排器状态
pub struct OrchestratorState {
    initialized: RwLock<bool>,
}

impl OrchestratorState {
    pub fn new() -> Self {
        Self {
            initialized: RwLock::new(false),
        }
    }
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 初始化命令
// ============================================================================

/// 初始化编排器
#[tauri::command]
pub async fn init_orchestrator(
    state: State<'_, OrchestratorState>,
    db: State<'_, DbConnection>,
) -> Result<(), String> {
    let mut initialized = state.initialized.write().await;
    if *initialized {
        return Ok(());
    }

    let orchestrator = init_global_orchestrator();
    *initialized = true;

    // 从数据库加载凭证并同步到 orchestrator
    let credentials = {
        let conn = db.lock().map_err(|e| format!("获取数据库连接失败: {e}"))?;
        ProviderPoolDao::get_all(&conn).map_err(|e| format!("获取凭证列表失败: {e}"))?
    };

    // 转换凭证格式
    let cred_infos: Vec<CredentialInfo> = credentials
        .iter()
        .filter(|c| !c.is_disabled && c.is_healthy)
        .map(|c| {
            // 从 credential 中提取支持的模型列表
            let supported_models = extract_supported_models(&c.credential);
            // 保存原始的 provider_type 字符串（如 "antigravity"、"kiro" 等）
            let original_provider_type = c.provider_type.to_string();

            CredentialInfo {
                id: c.uuid.clone(),
                provider_type: map_pool_provider_type(&original_provider_type),
                original_provider_type: Some(original_provider_type),
                supported_models,
                is_healthy: c.is_healthy,
                current_load: None,
            }
        })
        .collect();

    if !cred_infos.is_empty() {
        orchestrator.update_credentials(cred_infos).await;
        tracing::info!("已从凭证池同步 {} 个凭证到编排器", credentials.len());
    }

    tracing::info!("模型编排器已初始化");
    Ok(())
}

/// 从 credential 提取支持的模型列表
fn extract_supported_models(
    credential: &crate::models::provider_pool_model::CredentialData,
) -> Vec<String> {
    use crate::models::provider_pool_model::CredentialData;

    match credential {
        CredentialData::ClaudeKey { .. } | CredentialData::ClaudeOAuth { .. } => {
            vec![
                "claude-opus-4-5-20251101".to_string(),
                "claude-opus-4-20250514".to_string(),
                "claude-sonnet-4-5-20250929".to_string(),
                "claude-sonnet-4-20250514".to_string(),
                "claude-haiku-4-5-20251001".to_string(),
                "claude-3-7-sonnet-20250219".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
            ]
        }
        CredentialData::OpenAIKey { .. } => {
            vec![
                "gpt-5.2-codex".to_string(),
                "gpt-5.2".to_string(),
                "gpt-5.1-codex-max".to_string(),
                "gpt-5.1-codex".to_string(),
                "gpt-5.1-codex-mini".to_string(),
                "gpt-5.1".to_string(),
                "gpt-5-codex".to_string(),
                "gpt-5-codex-mini".to_string(),
                "gpt-5".to_string(),
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
            ]
        }
        CredentialData::GeminiOAuth { .. } => {
            vec![
                "gemini-3-pro-preview".to_string(),
                "gemini-3-flash-preview".to_string(),
                "gemini-2.5-pro".to_string(),
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-flash-lite".to_string(),
            ]
        }
        CredentialData::GeminiApiKey {
            excluded_models, ..
        } => {
            let all_models = vec![
                "gemini-3-pro-preview".to_string(),
                "gemini-3-flash-preview".to_string(),
                "gemini-2.5-pro".to_string(),
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-flash-lite".to_string(),
            ];
            all_models
                .into_iter()
                .filter(|m| !excluded_models.contains(m))
                .collect()
        }
        CredentialData::KiroOAuth { .. } => {
            vec![
                "claude-opus-4-5".to_string(),
                "claude-opus-4-5-20251101".to_string(),
                "claude-haiku-4-5".to_string(),
                "claude-sonnet-4-5".to_string(),
                "claude-sonnet-4-5-20250929".to_string(),
                "claude-sonnet-4-20250514".to_string(),
                "claude-3-7-sonnet-20250219".to_string(),
            ]
        }
        CredentialData::CodexOAuth { .. } => {
            vec!["codex-mini-latest".to_string()]
        }
        CredentialData::AntigravityOAuth { .. } => {
            vec![
                // Max 等级
                "gemini-3-pro-preview".to_string(),
                "gemini-3-pro-image-preview".to_string(),
                "gemini-claude-opus-4-5-thinking".to_string(),
                // Pro 等级
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-computer-use-preview-10-2025".to_string(),
                "gemini-claude-sonnet-4-5".to_string(),
                "gemini-claude-sonnet-4-5-thinking".to_string(),
                // Mini 等级
                "gemini-3-flash-preview".to_string(),
            ]
        }
        _ => vec![],
    }
}

/// 映射 PoolProviderType 到 orchestrator 的 ProviderType
fn map_pool_provider_type(pool_type: &str) -> ProviderType {
    match pool_type.to_lowercase().as_str() {
        "claude" | "claude_oauth" => ProviderType::Anthropic,
        "openai" => ProviderType::OpenAI,
        "gemini" | "gemini_api_key" | "gemini_oauth" => ProviderType::Google,
        "kiro" => ProviderType::Kiro,
        "codex" => ProviderType::OpenAI,
        "antigravity" => ProviderType::Antigravity,
        _ => ProviderType::Custom,
    }
}

/// 获取编排器配置
#[tauri::command]
pub async fn get_orchestrator_config() -> Result<OrchestratorConfig, String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    Ok(orchestrator.get_config().await)
}

/// 更新编排器配置
#[tauri::command]
pub async fn update_orchestrator_config(config: OrchestratorConfig) -> Result<(), String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    orchestrator.update_config(config).await;
    Ok(())
}

// ============================================================================
// 模型池命令
// ============================================================================

/// 获取模型池统计
#[tauri::command]
pub async fn get_pool_stats() -> Result<PoolStats, String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    Ok(orchestrator.get_pool_stats().await)
}

/// 获取指定等级的模型列表
#[tauri::command]
pub async fn get_tier_models(tier: String) -> Result<Vec<AvailableModel>, String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    let service_tier =
        ServiceTier::from_str(&tier).ok_or_else(|| format!("无效的服务等级: {tier}"))?;

    Ok(orchestrator.get_models(service_tier).await)
}

/// 获取所有可用模型
#[tauri::command]
pub async fn get_all_models() -> Result<Vec<AvailableModel>, String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    Ok(orchestrator.get_all_models().await)
}

// ============================================================================
// 凭证管理命令
// ============================================================================

/// 凭证信息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialInfoRequest {
    pub id: String,
    pub provider_type: String,
    pub supported_models: Vec<String>,
    pub is_healthy: bool,
    pub current_load: Option<u8>,
}

impl From<CredentialInfoRequest> for CredentialInfo {
    fn from(req: CredentialInfoRequest) -> Self {
        // 保存原始的 provider_type 字符串
        let original_provider_type = req.provider_type.clone();
        CredentialInfo {
            id: req.id,
            provider_type: ProviderType::from_str(&req.provider_type)
                .unwrap_or(ProviderType::Custom),
            original_provider_type: Some(original_provider_type),
            supported_models: req.supported_models,
            is_healthy: req.is_healthy,
            current_load: req.current_load,
        }
    }
}

/// 更新凭证列表
#[tauri::command]
pub async fn update_orchestrator_credentials(
    credentials: Vec<CredentialInfoRequest>,
) -> Result<(), String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    let creds: Vec<CredentialInfo> = credentials.into_iter().map(Into::into).collect();
    orchestrator.update_credentials(creds).await;

    Ok(())
}

/// 添加凭证
#[tauri::command]
pub async fn add_orchestrator_credential(credential: CredentialInfoRequest) -> Result<(), String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    orchestrator.add_credential(credential.into()).await;
    Ok(())
}

/// 移除凭证
#[tauri::command]
pub async fn remove_orchestrator_credential(credential_id: String) -> Result<(), String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    orchestrator.remove_credential(&credential_id).await;
    Ok(())
}

/// 标记凭证为不健康
#[tauri::command]
pub async fn mark_credential_unhealthy(
    model_id: String,
    credential_id: String,
) -> Result<(), String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    orchestrator.mark_unhealthy(&model_id, &credential_id).await;
    Ok(())
}

/// 标记凭证为健康
#[tauri::command]
pub async fn mark_credential_healthy(credential_id: String) -> Result<(), String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    orchestrator.mark_healthy(&credential_id).await;
    Ok(())
}

/// 更新凭证负载
#[tauri::command]
pub async fn update_credential_load(credential_id: String, load: u8) -> Result<(), String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    orchestrator.update_load(&credential_id, load).await;
    Ok(())
}

// ============================================================================
// 模型选择命令
// ============================================================================

/// 选择请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRequest {
    pub tier: String,
    pub task_hint: Option<String>,
    pub requires_vision: Option<bool>,
    pub requires_tools: Option<bool>,
    pub preferred_provider: Option<String>,
    pub excluded_models: Option<Vec<String>>,
    pub strategy_id: Option<String>,
}

/// 选择模型
#[tauri::command]
pub async fn select_model(request: SelectionRequest) -> Result<SelectionResult, String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    let tier = ServiceTier::from_str(&request.tier)
        .ok_or_else(|| format!("无效的服务等级: {}", request.tier))?;

    let mut ctx = SelectionContext::new(tier);

    if let Some(hint) = &request.task_hint {
        ctx.task_hint = match hint.to_lowercase().as_str() {
            "coding" => Some(TaskHint::Coding),
            "writing" => Some(TaskHint::Writing),
            "analysis" => Some(TaskHint::Analysis),
            "chat" => Some(TaskHint::Chat),
            "translation" => Some(TaskHint::Translation),
            "summarization" => Some(TaskHint::Summarization),
            "math" => Some(TaskHint::Math),
            _ => Some(TaskHint::Other),
        };
    }

    if let Some(vision) = request.requires_vision {
        ctx.requires_vision = vision;
    }

    if let Some(tools) = request.requires_tools {
        ctx.requires_tools = tools;
    }

    if let Some(provider) = request.preferred_provider {
        ctx.preferred_provider = Some(provider);
    }

    if let Some(excluded) = request.excluded_models {
        ctx.excluded_models = excluded;
    }

    let result = if let Some(strategy_id) = &request.strategy_id {
        orchestrator.select_with_strategy(strategy_id, &ctx).await
    } else {
        orchestrator.select(&ctx).await
    };

    result.map_err(|e| e.to_string())
}

/// 快速选择模型
#[tauri::command]
pub async fn quick_select_model() -> Result<SelectionResult, String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    orchestrator.quick_select().await.map_err(|e| e.to_string())
}

/// 为特定任务选择模型
#[tauri::command]
pub async fn select_model_for_task(tier: String, task: String) -> Result<SelectionResult, String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    let service_tier =
        ServiceTier::from_str(&tier).ok_or_else(|| format!("无效的服务等级: {tier}"))?;

    let task_hint = match task.to_lowercase().as_str() {
        "coding" => TaskHint::Coding,
        "writing" => TaskHint::Writing,
        "analysis" => TaskHint::Analysis,
        "chat" => TaskHint::Chat,
        "translation" => TaskHint::Translation,
        "summarization" => TaskHint::Summarization,
        "math" => TaskHint::Math,
        _ => TaskHint::Other,
    };

    orchestrator
        .select_for_task(service_tier, task_hint)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// 策略命令
// ============================================================================

/// 列出所有可用策略
#[tauri::command]
pub async fn list_strategies() -> Result<Vec<StrategyInfo>, String> {
    let orchestrator = get_global_orchestrator().ok_or("编排器未初始化")?;

    Ok(orchestrator.list_strategies().await)
}

/// 获取服务等级列表
#[tauri::command]
pub fn list_service_tiers() -> Vec<ServiceTierInfo> {
    ServiceTier::all()
        .iter()
        .map(|t| ServiceTierInfo {
            id: format!("{t:?}").to_lowercase(),
            display_name: t.display_name().to_string(),
            description: t.description().to_string(),
            level: t.level(),
        })
        .collect()
}

/// 服务等级信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTierInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub level: u8,
}

/// 获取任务类型列表
#[tauri::command]
pub fn list_task_hints() -> Vec<TaskHintInfo> {
    vec![
        TaskHintInfo {
            id: "coding".to_string(),
            display_name: "代码".to_string(),
            description: "代码生成、编辑、调试".to_string(),
        },
        TaskHintInfo {
            id: "writing".to_string(),
            display_name: "写作".to_string(),
            description: "文章、报告、创意写作".to_string(),
        },
        TaskHintInfo {
            id: "analysis".to_string(),
            display_name: "分析".to_string(),
            description: "数据分析、推理、研究".to_string(),
        },
        TaskHintInfo {
            id: "chat".to_string(),
            display_name: "对话".to_string(),
            description: "日常对话、问答".to_string(),
        },
        TaskHintInfo {
            id: "translation".to_string(),
            display_name: "翻译".to_string(),
            description: "语言翻译".to_string(),
        },
        TaskHintInfo {
            id: "summarization".to_string(),
            display_name: "摘要".to_string(),
            description: "文本摘要、总结".to_string(),
        },
        TaskHintInfo {
            id: "math".to_string(),
            display_name: "数学".to_string(),
            description: "数学计算、推理".to_string(),
        },
        TaskHintInfo {
            id: "other".to_string(),
            display_name: "其他".to_string(),
            description: "其他任务".to_string(),
        },
    ]
}

/// 任务类型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHintInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
}
