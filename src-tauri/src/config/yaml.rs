//! YAML 配置文件支持
//!
//! 提供 YAML 配置的加载、保存和管理功能

use super::types::Config;
use std::path::{Path, PathBuf};

/// 配置错误类型
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// 文件读取错误
    ReadError(String),
    /// 文件写入错误
    WriteError(String),
    /// YAML 解析错误
    ParseError(String),
    /// YAML 序列化错误
    SerializeError(String),
    /// 配置验证错误
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ReadError(msg) => write!(f, "配置读取错误: {}", msg),
            ConfigError::WriteError(msg) => write!(f, "配置写入错误: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "YAML 解析错误: {}", msg),
            ConfigError::SerializeError(msg) => write!(f, "YAML 序列化错误: {}", msg),
            ConfigError::ValidationError(msg) => write!(f, "配置验证错误: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// 配置管理器
///
/// 管理 YAML 配置文件的加载、保存和热重载
#[derive(Debug)]
pub struct ConfigManager {
    /// 当前配置
    config: Config,
    /// 配置文件路径
    config_path: PathBuf,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config: Config::default(),
            config_path,
        }
    }

    /// 从文件加载配置
    ///
    /// 如果文件不存在，返回默认配置
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let config = if path.exists() {
            let content =
                std::fs::read_to_string(path).map_err(|e| ConfigError::ReadError(e.to_string()))?;
            Self::parse_yaml(&content)?
        } else {
            Config::default()
        };

        Ok(Self {
            config,
            config_path: path.to_path_buf(),
        })
    }

    /// 从 YAML 字符串解析配置
    pub fn parse_yaml(yaml: &str) -> Result<Config, ConfigError> {
        serde_yaml::from_str(yaml).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// 将配置序列化为 YAML 字符串
    pub fn to_yaml(config: &Config) -> Result<String, ConfigError> {
        serde_yaml::to_string(config).map_err(|e| ConfigError::SerializeError(e.to_string()))
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), ConfigError> {
        self.save_to(&self.config_path)
    }

    /// 保存配置到指定路径
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::WriteError(e.to_string()))?;
        }

        let yaml = Self::to_yaml(&self.config)?;
        std::fs::write(path, yaml).map_err(|e| ConfigError::WriteError(e.to_string()))
    }

    /// 重新加载配置
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        let content = std::fs::read_to_string(&self.config_path)
            .map_err(|e| ConfigError::ReadError(e.to_string()))?;
        self.config = Self::parse_yaml(&content)?;
        Ok(())
    }

    /// 获取当前配置
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取可变配置引用
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 设置配置
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// 获取配置文件路径
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// 导出配置为 YAML 字符串
    ///
    /// # Arguments
    /// * `redact_secrets` - 是否脱敏敏感信息（API 密钥等）
    pub fn export(&self, redact_secrets: bool) -> Result<String, ConfigError> {
        if redact_secrets {
            let mut config = self.config.clone();
            // 脱敏 API 密钥
            config.server.api_key = "***REDACTED***".to_string();
            if config.providers.openai.api_key.is_some() {
                config.providers.openai.api_key = Some("***REDACTED***".to_string());
            }
            if config.providers.claude.api_key.is_some() {
                config.providers.claude.api_key = Some("***REDACTED***".to_string());
            }
            Self::to_yaml(&config)
        } else {
            Self::to_yaml(&self.config)
        }
    }

    /// 从 YAML 字符串导入配置
    ///
    /// # Arguments
    /// * `yaml` - YAML 配置字符串
    /// * `merge` - 是否合并到现有配置（true）或替换（false）
    pub fn import(&mut self, yaml: &str, merge: bool) -> Result<(), ConfigError> {
        let imported = Self::parse_yaml(yaml)?;

        if merge {
            // 合并配置：只更新导入配置中非默认的字段
            self.merge_config(imported);
        } else {
            // 替换配置
            self.config = imported;
        }

        Ok(())
    }

    /// 合并配置
    fn merge_config(&mut self, other: Config) {
        // 合并服务器配置
        if other.server != ServerConfig::default() {
            self.config.server = other.server;
        }

        // 合并 Provider 配置
        if other.providers.kiro.enabled || other.providers.kiro.credentials_path.is_some() {
            self.config.providers.kiro = other.providers.kiro;
        }
        if other.providers.gemini.enabled || other.providers.gemini.credentials_path.is_some() {
            self.config.providers.gemini = other.providers.gemini;
        }
        if other.providers.qwen.enabled || other.providers.qwen.credentials_path.is_some() {
            self.config.providers.qwen = other.providers.qwen;
        }
        if other.providers.openai.enabled || other.providers.openai.api_key.is_some() {
            self.config.providers.openai = other.providers.openai;
        }
        if other.providers.claude.enabled || other.providers.claude.api_key.is_some() {
            self.config.providers.claude = other.providers.claude;
        }

        // 合并路由配置
        if !other.routing.rules.is_empty() {
            self.config.routing.rules = other.routing.rules;
        }
        if !other.routing.model_aliases.is_empty() {
            self.config
                .routing
                .model_aliases
                .extend(other.routing.model_aliases);
        }
        if !other.routing.exclusions.is_empty() {
            self.config
                .routing
                .exclusions
                .extend(other.routing.exclusions);
        }
        if other.routing.default_provider != "kiro" {
            self.config.routing.default_provider = other.routing.default_provider;
        }

        // 合并重试配置
        if other.retry != RetrySettings::default() {
            self.config.retry = other.retry;
        }

        // 合并日志配置
        if other.logging != LoggingConfig::default() {
            self.config.logging = other.logging;
        }
    }

    /// 获取默认配置文件路径
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("proxycast")
            .join("config.yaml")
    }
}

use super::types::{LoggingConfig, RetrySettings, ServerConfig};

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new(Self::default_config_path())
    }
}

// ============ 向后兼容的 JSON 配置函数 ============

/// 获取 JSON 配置文件路径（向后兼容）
fn json_config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("proxycast")
        .join("config.json")
}

/// 加载配置（向后兼容）
///
/// 优先加载 YAML 配置，如果不存在则尝试加载 JSON 配置
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let yaml_path = ConfigManager::default_config_path();
    let json_path = json_config_path();

    // 优先尝试 YAML 配置
    if yaml_path.exists() {
        let content = std::fs::read_to_string(&yaml_path)?;
        let config = serde_yaml::from_str(&content)?;
        return Ok(config);
    }

    // 回退到 JSON 配置
    if json_path.exists() {
        let content = std::fs::read_to_string(&json_path)?;
        let config = serde_json::from_str(&content)?;
        return Ok(config);
    }

    // 都不存在，返回默认配置
    Ok(Config::default())
}

/// 保存配置（向后兼容，使用 JSON 格式）
pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let path = json_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// 保存配置为 YAML 格式
pub fn save_config_yaml(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let path = ConfigManager::default_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_yaml::to_string(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_parse_yaml_minimal() {
        let yaml = r#"
server:
  host: "0.0.0.0"
  port: 9000
  api_key: "test-key"
providers:
  kiro:
    enabled: true
"#;
        let config = ConfigManager::parse_yaml(yaml).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.server.api_key, "test-key");
        assert!(config.providers.kiro.enabled);
    }

    #[test]
    fn test_parse_yaml_full() {
        let yaml = r#"
server:
  host: "127.0.0.1"
  port: 8999
  api_key: "proxy_cast"
providers:
  kiro:
    enabled: true
    credentials_path: "~/.aws/sso/cache/kiro-auth-token.json"
    region: "us-east-1"
  gemini:
    enabled: false
  qwen:
    enabled: false
  openai:
    enabled: false
    base_url: "https://api.openai.com/v1"
  claude:
    enabled: false
routing:
  default_provider: "kiro"
  rules:
    - pattern: "claude-*"
      provider: "kiro"
      priority: 10
  model_aliases:
    gpt-4: "claude-sonnet-4-5-20250514"
  exclusions:
    gemini:
      - "*-preview"
retry:
  max_retries: 3
  base_delay_ms: 1000
  max_delay_ms: 30000
  auto_switch_provider: true
logging:
  enabled: true
  level: "info"
  retention_days: 7
"#;
        let config = ConfigManager::parse_yaml(yaml).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.routing.rules.len(), 1);
        assert_eq!(config.routing.rules[0].pattern, "claude-*");
        assert_eq!(
            config.routing.model_aliases.get("gpt-4"),
            Some(&"claude-sonnet-4-5-20250514".to_string())
        );
        assert_eq!(
            config.routing.exclusions.get("gemini").map(|v| v.len()),
            Some(1)
        );
    }

    #[test]
    fn test_to_yaml_roundtrip() {
        let config = Config::default();
        let yaml = ConfigManager::to_yaml(&config).unwrap();
        let parsed = ConfigManager::parse_yaml(&yaml).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_parse_yaml_with_defaults() {
        // 只提供部分配置，其他使用默认值
        let yaml = r#"
server:
  port: 9999
"#;
        let config = ConfigManager::parse_yaml(yaml).unwrap();
        assert_eq!(config.server.port, 9999);
        // 其他字段应使用默认值
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.retry.max_retries, 3);
    }

    #[test]
    fn test_export_redacted() {
        let mut config = Config::default();
        config.server.api_key = "secret-key".to_string();
        config.providers.openai.api_key = Some("openai-secret".to_string());

        let manager = ConfigManager {
            config,
            config_path: PathBuf::from("test.yaml"),
        };

        let exported = manager.export(true).unwrap();
        assert!(exported.contains("***REDACTED***"));
        assert!(!exported.contains("secret-key"));
        assert!(!exported.contains("openai-secret"));
    }

    #[test]
    fn test_export_not_redacted() {
        let mut config = Config::default();
        config.server.api_key = "secret-key".to_string();

        let manager = ConfigManager {
            config,
            config_path: PathBuf::from("test.yaml"),
        };

        let exported = manager.export(false).unwrap();
        assert!(exported.contains("secret-key"));
        assert!(!exported.contains("***REDACTED***"));
    }

    #[test]
    fn test_import_replace() {
        let mut manager = ConfigManager::default();
        manager.config.server.port = 1234;

        let yaml = r#"
server:
  port: 5678
"#;
        manager.import(yaml, false).unwrap();
        assert_eq!(manager.config.server.port, 5678);
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::ParseError("invalid yaml".to_string());
        assert!(err.to_string().contains("YAML 解析错误"));
        assert!(err.to_string().contains("invalid yaml"));
    }
}
