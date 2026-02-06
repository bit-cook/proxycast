use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// MCP 服务器配置（类型化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigTyped {
    /// 启动命令
    pub command: String,
    /// 命令参数
    #[serde(default)]
    pub args: Vec<String>,
    /// 环境变量
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// 工作目录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// 超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    30
}

impl Default for McpServerConfigTyped {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            timeout: 30,
        }
    }
}

/// 配置验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub server_config: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled_proxycast: bool,
    #[serde(default)]
    pub enabled_claude: bool,
    #[serde(default)]
    pub enabled_codex: bool,
    #[serde(default)]
    pub enabled_gemini: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

impl McpServer {
    #[allow(dead_code)]
    pub fn new(id: String, name: String, server_config: Value) -> Self {
        Self {
            id,
            name,
            server_config,
            description: None,
            enabled_proxycast: false,
            enabled_claude: false,
            enabled_codex: false,
            enabled_gemini: false,
            created_at: Some(chrono::Utc::now().timestamp()),
        }
    }

    /// 解析 server_config 为类型化配置
    ///
    /// 将 JSON Value 解析为 McpServerConfigTyped 结构。
    /// 如果解析失败，返回默认配置并尝试提取基本字段。
    pub fn parse_config(&self) -> McpServerConfigTyped {
        serde_json::from_value(self.server_config.clone()).unwrap_or_else(|_| {
            // 尝试手动提取字段
            McpServerConfigTyped {
                command: self
                    .server_config
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                args: self
                    .server_config
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                env: self
                    .server_config
                    .get("env")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default(),
                cwd: self
                    .server_config
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                timeout: self
                    .server_config
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30),
            }
        })
    }

    /// 验证服务器配置
    ///
    /// 检查配置是否有效，返回验证错误列表。
    /// 空列表表示配置有效。
    pub fn validate_config(&self) -> Vec<ConfigValidationError> {
        let mut errors = Vec::new();
        let config = self.parse_config();

        // 验证 command 不为空
        if config.command.trim().is_empty() {
            errors.push(ConfigValidationError {
                field: "command".to_string(),
                message: "启动命令不能为空".to_string(),
            });
        }

        // 验证 name 不为空
        if self.name.trim().is_empty() {
            errors.push(ConfigValidationError {
                field: "name".to_string(),
                message: "服务器名称不能为空".to_string(),
            });
        }

        // 验证 name 不包含特殊字符（用于工具名称前缀）
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            errors.push(ConfigValidationError {
                field: "name".to_string(),
                message: "服务器名称只能包含字母、数字、连字符和下划线".to_string(),
            });
        }

        // 验证 timeout 在合理范围内
        if config.timeout == 0 || config.timeout > 300 {
            errors.push(ConfigValidationError {
                field: "timeout".to_string(),
                message: "超时时间必须在 1-300 秒之间".to_string(),
            });
        }

        errors
    }

    /// 检查配置是否有效
    pub fn is_valid(&self) -> bool {
        self.validate_config().is_empty()
    }
}
