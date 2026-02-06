//! 凭证池桥接模块
//!
//! 将 ProxyCast 凭证池与 Aster Provider 系统连接
//! 支持从凭证池自动选择凭证并配置 Aster Provider
//!
//! ## 功能
//! - 从凭证池选择可用凭证
//! - 将凭证转换为 Aster Provider 配置
//! - 支持 OAuth 和 API Key 两种凭证类型
//! - 自动刷新过期的 OAuth Token

use crate::database::DbConnection;
use crate::models::provider_pool_model::{CredentialData, PoolProviderType, ProviderCredential};
use crate::services::api_key_provider_service::ApiKeyProviderService;
use crate::services::provider_pool_service::ProviderPoolService;
use aster::model::ModelConfig;
use aster::providers::base::Provider;
use std::sync::Arc;

/// 凭证桥接错误
#[derive(Debug, Clone)]
pub enum CredentialBridgeError {
    /// 没有可用凭证
    NoCredentials(String),
    /// 凭证类型不支持
    UnsupportedCredentialType(String),
    /// Provider 创建失败
    ProviderCreationFailed(String),
    /// Token 刷新失败
    TokenRefreshFailed(String),
    /// 数据库错误
    DatabaseError(String),
}

impl std::fmt::Display for CredentialBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCredentials(msg) => write!(f, "没有可用凭证: {msg}"),
            Self::UnsupportedCredentialType(msg) => write!(f, "不支持的凭证类型: {msg}"),
            Self::ProviderCreationFailed(msg) => write!(f, "Provider 创建失败: {msg}"),
            Self::TokenRefreshFailed(msg) => write!(f, "Token 刷新失败: {msg}"),
            Self::DatabaseError(msg) => write!(f, "数据库错误: {msg}"),
        }
    }
}

impl std::error::Error for CredentialBridgeError {}

/// Aster Provider 配置
#[derive(Debug, Clone)]
pub struct AsterProviderConfig {
    /// Provider 名称 (openai, anthropic, google 等)
    pub provider_name: String,
    /// 模型名称
    pub model_name: String,
    /// API Key
    pub api_key: Option<String>,
    /// Base URL
    pub base_url: Option<String>,
    /// 凭证 UUID（用于记录使用和健康状态）
    pub credential_uuid: String,
}

/// 凭证池桥接器
///
/// 负责从 ProxyCast 凭证池选择凭证并转换为 Aster Provider 配置
pub struct CredentialBridge {
    pool_service: ProviderPoolService,
    api_key_service: ApiKeyProviderService,
}

impl Default for CredentialBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialBridge {
    pub fn new() -> Self {
        Self {
            pool_service: ProviderPoolService::new(),
            api_key_service: ApiKeyProviderService::new(),
        }
    }

    /// 从凭证池选择凭证并创建 Aster Provider 配置
    ///
    /// # 参数
    /// - `db`: 数据库连接
    /// - `provider_type`: Provider 类型 (openai, anthropic, kiro, deepseek 等)
    /// - `model`: 模型名称
    ///
    /// # 返回
    /// 成功时返回 AsterProviderConfig，失败时返回错误
    pub async fn select_and_configure(
        &self,
        db: &DbConnection,
        provider_type: &str,
        model: &str,
    ) -> Result<AsterProviderConfig, CredentialBridgeError> {
        // 1. 从凭证池选择凭证
        // 将 provider_type 同时作为 provider_id_hint 传递，支持 60+ API Key Provider
        // 例如 "deepseek", "moonshot", "qwen" 等
        let credential = self
            .pool_service
            .select_credential_with_fallback(
                db,
                &self.api_key_service,
                provider_type,
                Some(model),
                Some(provider_type), // 传递 provider_id_hint 支持智能降级
                None,
            )
            .await
            .map_err(CredentialBridgeError::DatabaseError)?
            .ok_or_else(|| {
                CredentialBridgeError::NoCredentials(format!(
                    "没有找到 {provider_type} 类型的可用凭证"
                ))
            })?;

        // 2. 转换为 Aster Provider 配置，传递 provider_type 以便正确识别 Provider
        self.credential_to_config(&credential, model, provider_type, db)
            .await
    }

    /// 将 ProxyCast 凭证转换为 Aster Provider 配置
    async fn credential_to_config(
        &self,
        credential: &ProviderCredential,
        model: &str,
        provider_type_hint: &str,
        db: &DbConnection,
    ) -> Result<AsterProviderConfig, CredentialBridgeError> {
        tracing::info!(
            "[CredentialBridge] credential_to_config: provider_type_hint={}, credential_type={:?}",
            provider_type_hint,
            credential.provider_type
        );

        let (provider_name, api_key, base_url) = match &credential.credential {
            // OpenAI API Key - 根据 provider_type_hint 确定实际的 Provider
            CredentialData::OpenAIKey { api_key, base_url } => {
                // 使用 provider_type_hint 来确定 aster provider 名称
                let provider = map_provider_type_to_aster(provider_type_hint);
                tracing::info!(
                    "[CredentialBridge] OpenAIKey: provider_type_hint={} -> aster_provider={}",
                    provider_type_hint,
                    provider
                );
                (
                    provider.to_string(),
                    Some(api_key.clone()),
                    base_url.clone(),
                )
            }

            // Claude/Anthropic API Key
            CredentialData::ClaudeKey { api_key, base_url }
            | CredentialData::AnthropicKey { api_key, base_url } => (
                "anthropic".to_string(),
                Some(api_key.clone()),
                base_url.clone(),
            ),

            // Kiro OAuth - 需要获取 access_token
            CredentialData::KiroOAuth { creds_file_path } => {
                let token = self
                    .get_kiro_token(creds_file_path, db, &credential.uuid)
                    .await?;
                // Kiro 使用 CodeWhisperer API，映射到 bedrock provider
                ("bedrock".to_string(), Some(token), None)
            }

            // Gemini OAuth
            CredentialData::GeminiOAuth {
                creds_file_path, ..
            } => {
                let token = self.get_oauth_token(creds_file_path).await?;
                ("google".to_string(), Some(token), None)
            }

            // Gemini API Key
            CredentialData::GeminiApiKey {
                api_key, base_url, ..
            } => (
                "google".to_string(),
                Some(api_key.clone()),
                base_url.clone(),
            ),

            // Vertex AI
            CredentialData::VertexKey {
                api_key, base_url, ..
            } => (
                "gcpvertexai".to_string(),
                Some(api_key.clone()),
                base_url.clone(),
            ),

            // Codex OAuth
            CredentialData::CodexOAuth {
                creds_file_path,
                api_base_url,
            } => {
                let token = self.get_codex_token(creds_file_path).await?;
                ("codex".to_string(), Some(token), api_base_url.clone())
            }

            // Claude OAuth
            CredentialData::ClaudeOAuth { creds_file_path } => {
                let token = self.get_oauth_token(creds_file_path).await?;
                ("anthropic".to_string(), Some(token), None)
            }

            // Antigravity OAuth
            CredentialData::AntigravityOAuth {
                creds_file_path, ..
            } => {
                let token = self.get_oauth_token(creds_file_path).await?;
                ("google".to_string(), Some(token), None)
            }
        };

        Ok(AsterProviderConfig {
            provider_name,
            model_name: model.to_string(),
            api_key,
            base_url,
            credential_uuid: credential.uuid.clone(),
        })
    }

    /// 获取 Kiro OAuth Token
    async fn get_kiro_token(
        &self,
        creds_path: &str,
        _db: &DbConnection,
        _uuid: &str,
    ) -> Result<String, CredentialBridgeError> {
        use crate::providers::kiro::KiroProvider;

        let mut provider = KiroProvider::new();
        provider
            .load_credentials_from_path(creds_path)
            .await
            .map_err(|e| {
                CredentialBridgeError::TokenRefreshFailed(format!("加载 Kiro 凭证失败: {e}"))
            })?;

        // 检查 token 是否过期，如果过期则刷新
        if provider.is_token_expired() {
            tracing::info!("[CredentialBridge] Kiro token 已过期，尝试刷新");
            self.pool_service
                .refresh_kiro_token(creds_path)
                .await
                .map_err(CredentialBridgeError::TokenRefreshFailed)?;

            // 重新加载凭证
            provider
                .load_credentials_from_path(creds_path)
                .await
                .map_err(|e| {
                    CredentialBridgeError::TokenRefreshFailed(format!("重新加载凭证失败: {e}"))
                })?;
        }

        provider.credentials.access_token.ok_or_else(|| {
            CredentialBridgeError::TokenRefreshFailed("缺少 access_token".to_string())
        })
    }

    /// 获取通用 OAuth Token
    async fn get_oauth_token(&self, creds_path: &str) -> Result<String, CredentialBridgeError> {
        let content = std::fs::read_to_string(creds_path).map_err(|e| {
            CredentialBridgeError::TokenRefreshFailed(format!("读取凭证文件失败: {e}"))
        })?;

        let creds: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| CredentialBridgeError::TokenRefreshFailed(format!("解析凭证失败: {e}")))?;

        creds["access_token"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                CredentialBridgeError::TokenRefreshFailed("凭证中缺少 access_token".to_string())
            })
    }

    /// 获取 Codex OAuth Token
    async fn get_codex_token(&self, creds_path: &str) -> Result<String, CredentialBridgeError> {
        use crate::providers::codex::CodexProvider;

        let mut provider = CodexProvider::new();
        provider
            .load_credentials_from_path(creds_path)
            .await
            .map_err(|e| {
                CredentialBridgeError::TokenRefreshFailed(format!("加载 Codex 凭证失败: {e}"))
            })?;

        provider.ensure_valid_token().await.map_err(|e| {
            CredentialBridgeError::TokenRefreshFailed(format!("获取 Codex token 失败: {e}"))
        })
    }

    /// 记录凭证使用
    pub fn record_usage(&self, db: &DbConnection, uuid: &str) -> Result<(), CredentialBridgeError> {
        self.pool_service
            .record_usage(db, uuid)
            .map_err(CredentialBridgeError::DatabaseError)
    }

    /// 标记凭证为健康
    pub fn mark_healthy(
        &self,
        db: &DbConnection,
        uuid: &str,
        model: Option<&str>,
    ) -> Result<(), CredentialBridgeError> {
        self.pool_service
            .mark_healthy(db, uuid, model)
            .map_err(CredentialBridgeError::DatabaseError)
    }

    /// 标记凭证为不健康
    pub fn mark_unhealthy(
        &self,
        db: &DbConnection,
        uuid: &str,
        error: Option<&str>,
    ) -> Result<(), CredentialBridgeError> {
        self.pool_service
            .mark_unhealthy(db, uuid, error)
            .map_err(CredentialBridgeError::DatabaseError)
    }
}

/// 从 AsterProviderConfig 创建 Aster Provider
///
/// 设置环境变量并调用 aster::providers::create
pub async fn create_aster_provider(
    config: &AsterProviderConfig,
) -> Result<Arc<dyn Provider>, CredentialBridgeError> {
    // 设置环境变量
    set_provider_env_vars(config);

    // 创建 ModelConfig
    let model_config = ModelConfig::new(&config.model_name).map_err(|e| {
        CredentialBridgeError::ProviderCreationFailed(format!("创建 ModelConfig 失败: {e}"))
    })?;

    // 创建 Provider
    aster::providers::create(&config.provider_name, model_config)
        .await
        .map_err(|e| {
            CredentialBridgeError::ProviderCreationFailed(format!("创建 Provider 失败: {e}"))
        })
}

/// 设置 Provider 环境变量
fn set_provider_env_vars(config: &AsterProviderConfig) {
    tracing::info!(
        "[CredentialBridge] set_provider_env_vars: provider_name={}, has_api_key={}, base_url={:?}",
        config.provider_name,
        config.api_key.is_some(),
        config.base_url
    );

    let env_key = match config.provider_name.as_str() {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "google" => "GOOGLE_API_KEY",
        "bedrock" => "AWS_ACCESS_KEY_ID", // Bedrock 使用 AWS 凭证
        "gcpvertexai" => "GOOGLE_API_KEY",
        "codex" => "OPENAI_API_KEY", // Codex 兼容 OpenAI
        "deepseek" | "custom_deepseek" => "OPENAI_API_KEY", // DeepSeek 使用 OpenAI 兼容 API
        "groq" => "OPENAI_API_KEY",  // Groq 使用 OpenAI 兼容 API
        "mistral" => "OPENAI_API_KEY", // Mistral 使用 OpenAI 兼容 API
        "openrouter" => "OPENROUTER_API_KEY",
        _ => "OPENAI_API_KEY", // 默认使用 OpenAI 格式
    };

    tracing::info!("[CredentialBridge] 设置环境变量: {}=***", env_key);

    if let Some(api_key) = &config.api_key {
        std::env::set_var(env_key, api_key);
    }

    // 设置 base_url
    // Aster 的 OpenAI Provider 使用 OPENAI_HOST 环境变量
    if let Some(base_url) = &config.base_url {
        match config.provider_name.as_str() {
            "openai" => {
                // OpenAI 兼容的 Provider 使用 OPENAI_HOST
                std::env::set_var("OPENAI_HOST", base_url);
                tracing::info!("[CredentialBridge] 设置 OPENAI_HOST={}", base_url);
            }
            "anthropic" => {
                std::env::set_var("ANTHROPIC_BASE_URL", base_url);
            }
            _ => {
                // 其他 Provider 使用通用格式
                let base_url_key = format!(
                    "{}_BASE_URL",
                    config.provider_name.to_uppercase().replace('-', "_")
                );
                std::env::set_var(&base_url_key, base_url);
            }
        }
    }
}

/// Provider 类型映射
///
/// 将 ProxyCast PoolProviderType 映射到 Aster Provider 名称
pub fn map_pool_type_to_aster(pool_type: &PoolProviderType) -> &'static str {
    match pool_type {
        PoolProviderType::Kiro => "bedrock",
        PoolProviderType::Gemini => "google",
        PoolProviderType::Antigravity => "google",
        PoolProviderType::OpenAI => "openai",
        PoolProviderType::Claude => "anthropic",
        PoolProviderType::Anthropic => "anthropic",
        PoolProviderType::AnthropicCompatible => "anthropic",
        PoolProviderType::Vertex => "gcpvertexai",
        PoolProviderType::GeminiApiKey => "google",
        PoolProviderType::Codex => "codex",
        PoolProviderType::ClaudeOAuth => "anthropic",
        PoolProviderType::AzureOpenai => "azure",
        PoolProviderType::AwsBedrock => "bedrock",
        PoolProviderType::Ollama => "ollama",
    }
}

/// 将 provider_type 字符串映射到 Aster Provider 名称
///
/// 支持 60+ API Key Provider，包括 deepseek, moonshot, qwen 等
fn map_provider_type_to_aster(provider_type: &str) -> &'static str {
    match provider_type {
        // 标准 Provider
        "openai" => "openai",
        "anthropic" | "claude" => "anthropic",
        "google" | "gemini" => "google",
        "bedrock" | "kiro" => "bedrock",
        "gcpvertexai" | "vertex" => "gcpvertexai",
        "codex" => "codex",
        "azure" | "azure-openai" => "azure",
        "ollama" => "ollama",

        // DeepSeek - 使用 openai 兼容 provider（Aster 会通过 alias 映射）
        "deepseek" | "custom_deepseek" => "openai",

        // 其他 OpenAI 兼容 Provider - 使用 openai provider
        // 这些 Provider 都使用 OpenAI 兼容 API，通过 base_url 区分
        "groq" => "openai",
        "mistral" => "openai",
        "openrouter" => "openrouter",

        // 默认使用 openai（OpenAI 兼容格式）
        _ => "openai",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_pool_type_to_aster() {
        assert_eq!(map_pool_type_to_aster(&PoolProviderType::OpenAI), "openai");
        assert_eq!(
            map_pool_type_to_aster(&PoolProviderType::Claude),
            "anthropic"
        );
        assert_eq!(map_pool_type_to_aster(&PoolProviderType::Gemini), "google");
        assert_eq!(map_pool_type_to_aster(&PoolProviderType::Kiro), "bedrock");
    }

    #[test]
    fn test_credential_bridge_error_display() {
        let err = CredentialBridgeError::NoCredentials("test".to_string());
        assert!(err.to_string().contains("没有可用凭证"));
    }
}
