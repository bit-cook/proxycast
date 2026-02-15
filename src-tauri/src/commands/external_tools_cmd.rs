//! 外部 CLI 工具管理命令
//!
//! 管理 Codex CLI 等外部工具的状态检查和配置
//! 这些工具有自己的认证系统，不通过 ProxyCast 凭证池管理

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

/// Codex CLI 状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexCliStatus {
    /// CLI 是否已安装
    pub installed: bool,
    /// CLI 版本
    pub version: Option<String>,
    /// 是否已登录
    pub logged_in: bool,
    /// 登录方式（api_key 或 oauth）
    pub auth_type: Option<String>,
    /// API Key 前缀（如果使用 API Key 登录）
    pub api_key_prefix: Option<String>,
    /// 错误信息
    pub error: Option<String>,
}

/// 检查 Codex CLI 状态
#[tauri::command]
pub async fn check_codex_cli_status() -> Result<CodexCliStatus, String> {
    let mut status = CodexCliStatus::default();

    // 1. 检查 codex 命令是否存在
    let version_result = Command::new("codex")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match version_result {
        Ok(output) => {
            if output.status.success() {
                status.installed = true;
                let version_str = String::from_utf8_lossy(&output.stdout);
                // 解析版本号，格式通常是 "codex x.y.z" 或直接 "x.y.z"
                status.version = Some(version_str.trim().to_string());
            } else {
                status.error = Some("Codex CLI 未正确安装".to_string());
                return Ok(status);
            }
        }
        Err(e) => {
            status.error = Some(format!(
                "Codex CLI 未安装。请运行: npm i -g @openai/codex\n错误: {e}"
            ));
            return Ok(status);
        }
    }

    // 2. 检查登录状态
    let login_result = Command::new("codex")
        .args(["login", "status"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match login_result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}");

            tracing::debug!("[CodexCli] login status output: {}", combined);

            // 解析登录状态
            // 示例输出: "Logged in using an API key - cr_4453c***0b3a7"
            // 或: "Not logged in"
            if combined.contains("Logged in") {
                status.logged_in = true;

                if combined.contains("API key") || combined.contains("api key") {
                    status.auth_type = Some("api_key".to_string());
                    // 提取 API Key 前缀
                    if let Some(key_part) = combined.split('-').next_back() {
                        let key = key_part.trim();
                        if !key.is_empty() {
                            status.api_key_prefix = Some(key.to_string());
                        }
                    }
                } else if combined.contains("OAuth") || combined.contains("oauth") {
                    status.auth_type = Some("oauth".to_string());
                } else {
                    status.auth_type = Some("unknown".to_string());
                }
            } else {
                status.logged_in = false;
            }
        }
        Err(e) => {
            tracing::warn!("[CodexCli] 检查登录状态失败: {}", e);
            // 不设置 error，因为 CLI 已安装，只是无法检查登录状态
        }
    }

    Ok(status)
}

/// 打开 Codex CLI 登录（在终端中执行）
#[tauri::command]
pub async fn open_codex_cli_login() -> Result<String, String> {
    // 返回登录命令，让前端在终端中执行
    Ok("codex login".to_string())
}

/// 打开 Codex CLI 登出
#[tauri::command]
pub async fn open_codex_cli_logout() -> Result<String, String> {
    Ok("codex logout".to_string())
}

/// 外部工具列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTool {
    /// 工具 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 是否已安装
    pub installed: bool,
    /// 是否已配置/登录
    pub configured: bool,
    /// 安装命令
    pub install_command: String,
    /// 配置命令
    pub config_command: String,
    /// 文档链接
    pub doc_url: String,
}

/// 获取外部工具列表
#[tauri::command]
pub async fn get_external_tools() -> Result<Vec<ExternalTool>, String> {
    let mut tools = Vec::new();

    // Codex CLI
    let codex_status = check_codex_cli_status().await.unwrap_or_default();
    tools.push(ExternalTool {
        id: "codex-cli".to_string(),
        name: "Codex CLI".to_string(),
        description: "OpenAI Codex 命令行工具，支持 Agent 模式和工具调用".to_string(),
        installed: codex_status.installed,
        configured: codex_status.logged_in,
        install_command: "npm i -g @openai/codex".to_string(),
        config_command: "codex login".to_string(),
        doc_url: "https://github.com/openai/codex".to_string(),
    });

    // 可以在这里添加更多外部工具...

    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_codex_cli_status() {
        // 这个测试依赖于本地环境
        let status = check_codex_cli_status().await;
        assert!(status.is_ok());
        let status = status.unwrap();
        println!("Codex CLI Status: {:?}", status);
    }
}
