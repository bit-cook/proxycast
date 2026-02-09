//! Browser Tool 包装器
//!
//! 将 Playwright MCP Server 的工具映射到 Aster 工具系统
//! 提供浏览器自动化能力,包括导航、快照、点击、输入等操作

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

/// Browser Tool 错误类型
#[derive(Debug, Error)]
pub enum BrowserToolError {
    #[error("MCP 客户端未初始化")]
    ClientNotInitialized,

    #[error("工具调用失败: {0}")]
    ToolCallFailed(String),

    #[error("参数序列化失败: {0}")]
    SerializationError(String),

    #[error("MCP 错误: {0}")]
    McpError(String),
}

/// Browser Tool 动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    /// 导航到 URL
    Navigate { url: String },
    /// 获取页面快照
    Snapshot,
    /// 点击元素
    Click { ref_id: String },
    /// 输入文本
    Type { ref_id: String, text: String },
    /// 截图
    Screenshot { filename: Option<String> },
}

/// Browser Tool 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserToolResult {
    /// 是否成功
    pub success: bool,
    /// 输出内容
    pub output: String,
    /// 错误信息
    pub error: Option<String>,
}

/// Browser Tool 包装器
///
/// 提供对 Playwright MCP Server 工具的高级封装
pub struct BrowserTool {
    /// MCP 客户端
    mcp_client: Arc<Mutex<Option<Box<dyn aster::agents::mcp_client::McpClientTrait>>>>,
}

impl BrowserTool {
    /// 创建新的 Browser Tool 实例
    pub fn new() -> Self {
        Self {
            mcp_client: Arc::new(Mutex::new(None)),
        }
    }

    /// 设置 MCP 客户端
    pub async fn set_mcp_client(&self, client: Box<dyn aster::agents::mcp_client::McpClientTrait>) {
        let mut guard = self.mcp_client.lock().await;
        *guard = Some(client);
    }

    /// 执行浏览器动作
    ///
    /// # 参数
    /// - `action`: 浏览器动作
    ///
    /// # 返回
    /// 返回工具执行结果
    pub async fn execute(
        &self,
        action: BrowserAction,
    ) -> Result<BrowserToolResult, BrowserToolError> {
        let client_guard = self.mcp_client.lock().await;
        let client = client_guard
            .as_ref()
            .ok_or(BrowserToolError::ClientNotInitialized)?;

        // 根据动作类型调用对应的 MCP 工具
        match action {
            BrowserAction::Navigate { url } => {
                self.call_mcp_tool(
                    client,
                    "browser_navigate",
                    serde_json::json!({ "url": url }),
                )
                .await
            }
            BrowserAction::Snapshot => {
                self.call_mcp_tool(client, "browser_snapshot", serde_json::json!({}))
                    .await
            }
            BrowserAction::Click { ref_id } => {
                self.call_mcp_tool(
                    client,
                    "browser_click",
                    serde_json::json!({ "ref": ref_id }),
                )
                .await
            }
            BrowserAction::Type { ref_id, text } => {
                self.call_mcp_tool(
                    client,
                    "browser_type",
                    serde_json::json!({ "ref": ref_id, "text": text }),
                )
                .await
            }
            BrowserAction::Screenshot { filename } => {
                let mut args = serde_json::json!({ "type": "png" });
                if let Some(name) = filename {
                    args["filename"] = Value::String(name);
                }
                self.call_mcp_tool(client, "browser_take_screenshot", args)
                    .await
            }
        }
    }

    /// 调用 MCP 工具
    async fn call_mcp_tool(
        &self,
        client: &Box<dyn aster::agents::mcp_client::McpClientTrait>,
        tool_name: &str,
        arguments: Value,
    ) -> Result<BrowserToolResult, BrowserToolError> {
        // 将 Value 转换为 JsonObject
        let args = match arguments {
            Value::Object(map) => Some(map),
            _ => None,
        };

        // 创建取消令牌
        let cancel_token = tokio_util::sync::CancellationToken::new();

        // 调用工具
        let result = client
            .call_tool(tool_name, args, cancel_token)
            .await
            .map_err(|e| BrowserToolError::McpError(format!("{:?}", e)))?;

        // 转换结果
        let is_error = result.is_error.unwrap_or(false);
        let output = result
            .content
            .into_iter()
            .map(|c| match c.raw {
                rmcp::model::RawContent::Text(text) => text.text,
                rmcp::model::RawContent::Image(img) => {
                    format!("[Image: {}]", img.mime_type)
                }
                _ => "[Unknown content]".to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(BrowserToolResult {
            success: !is_error,
            output: output.clone(),
            error: if is_error { Some(output) } else { None },
        })
    }

    /// 导航到 URL
    pub async fn navigate(&self, url: &str) -> Result<BrowserToolResult, BrowserToolError> {
        self.execute(BrowserAction::Navigate {
            url: url.to_string(),
        })
        .await
    }

    /// 获取页面快照
    pub async fn snapshot(&self) -> Result<BrowserToolResult, BrowserToolError> {
        self.execute(BrowserAction::Snapshot).await
    }

    /// 点击元素
    pub async fn click(&self, ref_id: &str) -> Result<BrowserToolResult, BrowserToolError> {
        self.execute(BrowserAction::Click {
            ref_id: ref_id.to_string(),
        })
        .await
    }

    /// 输入文本
    pub async fn type_text(
        &self,
        ref_id: &str,
        text: &str,
    ) -> Result<BrowserToolResult, BrowserToolError> {
        self.execute(BrowserAction::Type {
            ref_id: ref_id.to_string(),
            text: text.to_string(),
        })
        .await
    }

    /// 截图
    pub async fn screenshot(
        &self,
        filename: Option<String>,
    ) -> Result<BrowserToolResult, BrowserToolError> {
        self.execute(BrowserAction::Screenshot { filename }).await
    }
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_tool_creation() {
        let tool = BrowserTool::new();
        assert!(tool.mcp_client.lock().await.is_none());
    }

    #[test]
    fn test_browser_action_serialization() {
        let action = BrowserAction::Navigate {
            url: "https://example.com".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("navigate"));
        assert!(json.contains("https://example.com"));
    }

    #[test]
    fn test_browser_tool_result() {
        let result = BrowserToolResult {
            success: true,
            output: "Page loaded".to_string(),
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.output, "Page loaded");
        assert!(result.error.is_none());
    }
}
