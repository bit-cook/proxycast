//! Provider 类型定义
//!
//! 包含 Provider 类型枚举和相关实现。

use serde::{Deserialize, Serialize};

/// Provider 类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Kiro,
    Gemini,
    #[serde(rename = "openai")]
    OpenAI,
    Claude,
    #[serde(rename = "claude_oauth")]
    ClaudeOAuth,
    /// Anthropic 兼容格式（支持 system 数组格式等变体）
    #[serde(rename = "anthropic_compatible")]
    AnthropicCompatible,
    Antigravity,
    Vertex,
    #[serde(rename = "gemini_api_key")]
    GeminiApiKey,
    Codex,
    // API Key Provider 类型
    Anthropic,
    #[serde(rename = "azure_openai")]
    AzureOpenai,
    #[serde(rename = "aws_bedrock")]
    AwsBedrock,
    Ollama,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Kiro => write!(f, "kiro"),
            ProviderType::Gemini => write!(f, "gemini"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Claude => write!(f, "claude"),
            ProviderType::ClaudeOAuth => write!(f, "claude_oauth"),
            ProviderType::AnthropicCompatible => write!(f, "anthropic_compatible"),
            ProviderType::Antigravity => write!(f, "antigravity"),
            ProviderType::Vertex => write!(f, "vertex"),
            ProviderType::GeminiApiKey => write!(f, "gemini_api_key"),
            ProviderType::Codex => write!(f, "codex"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::AzureOpenai => write!(f, "azure_openai"),
            ProviderType::AwsBedrock => write!(f, "aws_bedrock"),
            ProviderType::Ollama => write!(f, "ollama"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "kiro" => Ok(ProviderType::Kiro),
            "gemini" => Ok(ProviderType::Gemini),
            "openai" => Ok(ProviderType::OpenAI),
            "claude" => Ok(ProviderType::Claude),
            "claude_oauth" => Ok(ProviderType::ClaudeOAuth),
            "anthropic_compatible" | "anthropic-compatible" => {
                Ok(ProviderType::AnthropicCompatible)
            }
            "antigravity" => Ok(ProviderType::Antigravity),
            "vertex" => Ok(ProviderType::Vertex),
            "gemini_api_key" => Ok(ProviderType::GeminiApiKey),
            "codex" => Ok(ProviderType::Codex),
            "anthropic" => Ok(ProviderType::Anthropic),
            "azure_openai" | "azure-openai" => Ok(ProviderType::AzureOpenai),
            "aws_bedrock" | "aws-bedrock" => Ok(ProviderType::AwsBedrock),
            "ollama" => Ok(ProviderType::Ollama),
            _ => Err(format!("Invalid provider: {s}")),
        }
    }
}

/// Antigravity 支持的模型列表（fallback，当无法从 models 仓库获取时使用）
pub const ANTIGRAVITY_MODELS_FALLBACK: &[&str] = &[
    "gemini-2.5-computer-use-preview-10-2025",
    "gemini-3-pro-image-preview",
    "gemini-3-pro-preview",
    "gemini-3-flash-preview",
    "gemini-2.5-flash-preview",
    "gemini-2.5-flash",
    "gemini-2.5-pro",
    "gemini-3-flash",
    "gemini-3-pro-high",
    "gemini-3-pro-low",
    "gemini-claude-sonnet-4-5",
    "gemini-claude-sonnet-4-5-thinking",
    "gemini-claude-opus-4-5-thinking",
    "claude-sonnet-4-5",
    "claude-sonnet-4-5-thinking",
    "claude-opus-4-5-thinking",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!("kiro".parse::<ProviderType>().unwrap(), ProviderType::Kiro);
        assert_eq!(
            "gemini".parse::<ProviderType>().unwrap(),
            ProviderType::Gemini
        );
        assert_eq!(
            "openai".parse::<ProviderType>().unwrap(),
            ProviderType::OpenAI
        );
        assert_eq!(
            "claude".parse::<ProviderType>().unwrap(),
            ProviderType::Claude
        );
        assert_eq!(
            "vertex".parse::<ProviderType>().unwrap(),
            ProviderType::Vertex
        );
        assert_eq!(
            "gemini_api_key".parse::<ProviderType>().unwrap(),
            ProviderType::GeminiApiKey
        );
        assert_eq!("KIRO".parse::<ProviderType>().unwrap(), ProviderType::Kiro);
        assert_eq!(
            "Gemini".parse::<ProviderType>().unwrap(),
            ProviderType::Gemini
        );
        assert_eq!(
            "VERTEX".parse::<ProviderType>().unwrap(),
            ProviderType::Vertex
        );
        assert!("invalid".parse::<ProviderType>().is_err());
    }

    #[test]
    fn test_provider_type_display() {
        assert_eq!(ProviderType::Kiro.to_string(), "kiro");
        assert_eq!(ProviderType::Gemini.to_string(), "gemini");
        assert_eq!(ProviderType::OpenAI.to_string(), "openai");
        assert_eq!(ProviderType::Claude.to_string(), "claude");
        assert_eq!(ProviderType::Vertex.to_string(), "vertex");
        assert_eq!(ProviderType::GeminiApiKey.to_string(), "gemini_api_key");
    }

    #[test]
    fn test_provider_type_serde() {
        assert_eq!(
            serde_json::to_string(&ProviderType::Kiro).unwrap(),
            "\"kiro\""
        );
        assert_eq!(
            serde_json::to_string(&ProviderType::OpenAI).unwrap(),
            "\"openai\""
        );
        assert_eq!(
            serde_json::from_str::<ProviderType>("\"kiro\"").unwrap(),
            ProviderType::Kiro
        );
        assert_eq!(
            serde_json::from_str::<ProviderType>("\"openai\"").unwrap(),
            ProviderType::OpenAI
        );
    }
}
