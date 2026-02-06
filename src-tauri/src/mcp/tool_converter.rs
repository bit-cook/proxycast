//! MCP 工具格式转换器
//!
//! 本模块提供 MCP 工具定义与各 LLM Provider 格式之间的转换：
//! - OpenAI function calling 格式
//! - Anthropic tool use 格式
//! - Gemini function declaration 格式

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::types::{McpToolCall, McpToolDefinition};

// ============================================================================
// OpenAI 格式
// ============================================================================

/// OpenAI 工具格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAIFunction,
}

/// OpenAI 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// OpenAI 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAIFunctionCall,
}

/// OpenAI 函数调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String,
}

// ============================================================================
// Anthropic 格式
// ============================================================================

/// Anthropic 工具格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Anthropic 工具使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicToolUse {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

// ============================================================================
// Gemini 格式
// ============================================================================

/// Gemini 函数声明格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: GeminiParameters,
}

/// Gemini 参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: serde_json::Value,
    pub required: Vec<String>,
}

// ============================================================================
// 转换器实现
// ============================================================================

/// MCP 工具格式转换器
pub struct ToolConverter;

impl ToolConverter {
    /// 转换为 OpenAI function calling 格式
    pub fn to_openai(tools: &[McpToolDefinition]) -> Vec<OpenAITool> {
        tools
            .iter()
            .map(|tool| OpenAITool {
                tool_type: "function".to_string(),
                function: OpenAIFunction {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: tool.input_schema.clone(),
                },
            })
            .collect()
    }

    /// 转换为 Anthropic tool use 格式
    pub fn to_anthropic(tools: &[McpToolDefinition]) -> Vec<AnthropicTool> {
        tools
            .iter()
            .map(|tool| AnthropicTool {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
            })
            .collect()
    }

    /// 转换为 Gemini function declaration 格式
    pub fn to_gemini(tools: &[McpToolDefinition]) -> Vec<GeminiFunctionDeclaration> {
        tools
            .iter()
            .map(|tool| {
                // 从 input_schema 提取 properties 和 required
                let properties = tool
                    .input_schema
                    .get("properties")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                let required = tool
                    .input_schema
                    .get("required")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                GeminiFunctionDeclaration {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: GeminiParameters {
                        param_type: "object".to_string(),
                        properties,
                        required,
                    },
                }
            })
            .collect()
    }

    /// 从 OpenAI tool call 转换回 MCP 格式
    pub fn from_openai_call(call: &OpenAIToolCall) -> McpToolCall {
        let arguments =
            serde_json::from_str(&call.function.arguments).unwrap_or(serde_json::json!({}));

        McpToolCall {
            name: call.function.name.clone(),
            arguments,
        }
    }

    /// 从 Anthropic tool use 转换回 MCP 格式
    pub fn from_anthropic_use(use_: &AnthropicToolUse) -> McpToolCall {
        McpToolCall {
            name: use_.name.clone(),
            arguments: use_.input.clone(),
        }
    }
}
