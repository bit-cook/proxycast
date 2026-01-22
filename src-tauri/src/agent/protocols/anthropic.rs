//! Anthropic 协议实现
//!
//! 实现 Anthropic Messages API 协议
//! 适用于 Claude、Claude OAuth 等 Anthropic 服务

use super::Protocol;
use crate::agent::parsers::AnthropicSSEParser;
use crate::agent::types::{
    AgentConfig, AgentMessage, ContentPart, ImageData, MessageContent, StreamEvent, StreamResult,
};
use crate::models::anthropic::AnthropicMessage;
use crate::models::openai::Tool;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Anthropic Messages API 请求
#[derive(Debug, Serialize)]
struct AnthropicMessagesRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

/// Anthropic 工具定义
#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

/// Anthropic 协议处理器
pub struct AnthropicProtocol {
    /// 是否使用数组格式的 system 字段
    /// 标准 Anthropic: system: "prompt"
    /// 兼容格式: system: [{"type": "text", "text": "prompt"}]
    use_array_system_format: bool,
}

impl AnthropicProtocol {
    /// 创建标准 Anthropic 协议处理器
    pub fn new() -> Self {
        Self {
            use_array_system_format: false,
        }
    }

    /// 创建兼容格式的 Anthropic 协议处理器
    pub fn with_array_system_format() -> Self {
        Self {
            use_array_system_format: true,
        }
    }

    /// 构建 system 字段
    fn build_system_field(&self, prompt: &str) -> serde_json::Value {
        if self.use_array_system_format {
            serde_json::json!([{
                "type": "text",
                "text": prompt
            }])
        } else {
            serde_json::json!(prompt)
        }
    }

    /// 将 OpenAI Tool 转换为 Anthropic Tool
    fn convert_tools(tools: Option<&[Tool]>) -> Option<Vec<AnthropicTool>> {
        tools.map(|t| {
            t.iter()
                .filter_map(|tool| match tool {
                    Tool::Function { function } => Some(AnthropicTool {
                        name: function.name.clone(),
                        description: function.description.clone().unwrap_or_default(),
                        input_schema: function.parameters.clone().unwrap_or(serde_json::json!({
                            "type": "object",
                            "properties": {}
                        })),
                    }),
                    // WebSearch 工具不支持转换为 Anthropic 格式，跳过
                    Tool::WebSearch | Tool::WebSearch20250305 => None,
                })
                .collect()
        })
    }

    /// 将 AgentMessage 转换为 Anthropic Message
    fn convert_to_anthropic_message(msg: &AgentMessage) -> AnthropicMessage {
        let content = match &msg.content {
            MessageContent::Text(text) => {
                // 处理工具结果消息
                if msg.role == "tool" {
                    // Anthropic 使用 tool_result content block
                    if let Some(tool_call_id) = &msg.tool_call_id {
                        serde_json::json!([{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": text
                        }])
                    } else {
                        // 如果没有 tool_call_id，这可能是一个错误的工具结果消息
                        warn!("[AnthropicProtocol] 工具结果消息缺少 tool_call_id");
                        serde_json::json!(text)
                    }
                } else if msg.role == "assistant" {
                    // 处理 assistant 消息
                    let mut blocks = Vec::new();

                    if !text.is_empty() {
                        blocks.push(serde_json::json!({
                            "type": "text",
                            "text": text
                        }));
                    }

                    // 添加工具调用
                    if let Some(tool_calls) = &msg.tool_calls {
                        for tc in tool_calls {
                            let input: serde_json::Value =
                                serde_json::from_str(&tc.function.arguments)
                                    .unwrap_or(serde_json::json!({}));
                            blocks.push(serde_json::json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.function.name,
                                "input": input
                            }));
                        }
                    }

                    if blocks.is_empty() {
                        serde_json::json!("")
                    } else if blocks.len() == 1 && msg.tool_calls.is_none() {
                        serde_json::json!(text)
                    } else {
                        serde_json::json!(blocks)
                    }
                } else {
                    serde_json::json!(text)
                }
            }
            MessageContent::Parts(parts) => {
                let blocks: Vec<serde_json::Value> = parts
                    .iter()
                    .map(|p| match p {
                        ContentPart::Text { text } => serde_json::json!({
                            "type": "text",
                            "text": text
                        }),
                        ContentPart::ImageUrl { image_url } => {
                            // 解析 data URL
                            if let Some(rest) = image_url.url.strip_prefix("data:") {
                                if let Some(comma_idx) = rest.find(',') {
                                    let media_type = rest[..comma_idx]
                                        .strip_suffix(";base64")
                                        .unwrap_or(&rest[..comma_idx]);
                                    let data = &rest[comma_idx + 1..];
                                    return serde_json::json!({
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": media_type,
                                            "data": data
                                        }
                                    });
                                }
                            }
                            // 普通 URL
                            serde_json::json!({
                                "type": "image",
                                "source": {
                                    "type": "url",
                                    "url": image_url.url
                                }
                            })
                        }
                    })
                    .collect();
                serde_json::json!(blocks)
            }
        };

        // Anthropic 没有 "tool" 角色，需要转换为 "user"
        let role = if msg.role == "tool" {
            "user".to_string()
        } else {
            msg.role.clone()
        };

        AnthropicMessage { role, content }
    }

    /// 构建消息列表
    fn build_messages(
        &self,
        history: &[AgentMessage],
        user_message: &str,
        images: Option<&[ImageData]>,
        config: &AgentConfig,
    ) -> (Vec<AnthropicMessage>, Option<serde_json::Value>) {
        let mut messages = Vec::new();

        // 系统提示词（Anthropic 使用单独的 system 字段）
        let system_prompt = config
            .system_prompt
            .as_ref()
            .map(|s| self.build_system_field(s));

        // 验证和修复消息序列中的 tool_use/tool_result 配对
        let validated_history = self.validate_tool_message_pairs(history);

        // 添加历史消息（跳过 system 消息）
        for msg in &validated_history {
            if msg.role == "system" {
                continue;
            }
            messages.push(Self::convert_to_anthropic_message(msg));
        }

        // 添加当前用户消息
        let user_content = if let Some(imgs) = images {
            let mut parts = vec![serde_json::json!({
                "type": "text",
                "text": user_message
            })];

            for img in imgs {
                parts.push(serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": img.media_type,
                        "data": img.data
                    }
                }));
            }
            serde_json::json!(parts)
        } else {
            serde_json::json!(user_message)
        };

        messages.push(AnthropicMessage {
            role: "user".to_string(),
            content: user_content,
        });

        (messages, system_prompt)
    }

    /// 从历史构建消息（不添加新用户消息）
    fn build_messages_from_history(
        &self,
        history: &[AgentMessage],
        config: &AgentConfig,
    ) -> (Vec<AnthropicMessage>, Option<serde_json::Value>) {
        let mut messages = Vec::new();

        // 系统提示词
        let system_prompt = config
            .system_prompt
            .as_ref()
            .map(|s| self.build_system_field(s));

        // 验证和修复消息序列中的 tool_use/tool_result 配对
        let validated_history = self.validate_tool_message_pairs(history);

        // 添加所有历史消息（跳过 system）
        for msg in &validated_history {
            if msg.role == "system" {
                continue;
            }
            messages.push(Self::convert_to_anthropic_message(msg));
        }

        (messages, system_prompt)
    }

    /// 验证并修复消息序列中的 tool_use/tool_result 配对
    ///
    /// Claude API 要求每个 tool_use 都必须紧跟一个对应的 tool_result
    fn validate_tool_message_pairs(&self, history: &[AgentMessage]) -> Vec<AgentMessage> {
        let mut validated_messages = Vec::new();
        let mut pending_tool_calls: std::collections::HashMap<String, bool> =
            std::collections::HashMap::new();

        for msg in history {
            match msg.role.as_str() {
                "assistant" => {
                    // 检查是否有工具调用
                    if let Some(tool_calls) = &msg.tool_calls {
                        for tc in tool_calls {
                            pending_tool_calls.insert(tc.id.clone(), false);
                        }
                    }
                    validated_messages.push(msg.clone());
                }
                "tool" => {
                    // 检查工具结果是否有对应的工具调用
                    if let Some(tool_call_id) = &msg.tool_call_id {
                        if pending_tool_calls.contains_key(tool_call_id) {
                            pending_tool_calls.insert(tool_call_id.clone(), true);
                            validated_messages.push(msg.clone());
                        } else {
                            warn!(
                                "[AnthropicProtocol] 发现孤立的工具结果消息，tool_call_id: {}",
                                tool_call_id
                            );
                            // 跳过孤立的工具结果消息
                        }
                    } else {
                        warn!("[AnthropicProtocol] 工具结果消息缺少 tool_call_id");
                        // 跳过无效的工具结果消息
                    }
                }
                _ => {
                    validated_messages.push(msg.clone());
                }
            }
        }

        // 检查是否有未配对的工具调用
        for (tool_call_id, has_result) in &pending_tool_calls {
            if !has_result {
                warn!(
                    "[AnthropicProtocol] 发现未配对的工具调用，tool_call_id: {}，添加默认工具结果",
                    tool_call_id
                );
                // 为未配对的工具调用添加默认结果
                let default_result = AgentMessage {
                    role: "tool".to_string(),
                    content: MessageContent::Text("工具执行超时或失败".to_string()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    tool_calls: None,
                    tool_call_id: Some(tool_call_id.clone()),
                    reasoning_content: None,
                };
                validated_messages.push(default_result);
            }
        }

        validated_messages
    }

    /// 处理 SSE 流
    async fn process_stream(
        response: reqwest::Response,
        tx: mpsc::Sender<StreamEvent>,
        send_done: bool,
    ) -> Result<StreamResult, String> {
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut parser = AnthropicSSEParser::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buffer.push_str(&text);

                    // 处理完整的 SSE 事件
                    while let Some(pos) = buffer.find("\n\n") {
                        let event_block = buffer[..pos].to_string();
                        buffer = buffer[pos + 2..].to_string();

                        // 提取 event 类型和 data
                        let mut event_type = String::new();
                        let mut data = String::new();

                        for line in event_block.lines() {
                            if let Some(e) = line.strip_prefix("event: ") {
                                event_type = e.to_string();
                            } else if let Some(d) = line.strip_prefix("data: ") {
                                data = d.to_string();
                            }
                        }

                        if data.is_empty() {
                            continue;
                        }

                        debug!(
                            "[AnthropicProtocol] SSE event={}, data={}",
                            event_type, data
                        );
                        let result = parser.parse_data(&data);

                        // 发送工具开始事件
                        if let Some((tool_id, tool_name)) = result.tool_start {
                            let _ = tx
                                .send(StreamEvent::ToolStart {
                                    tool_name,
                                    tool_id,
                                    arguments: None,
                                })
                                .await;
                        }

                        // 发送文本增量
                        if let Some(text) = result.text_delta {
                            let _ = tx.send(StreamEvent::TextDelta { text }).await;
                        }

                        // 检查是否完成
                        if result.is_done {
                            let full_content = parser.get_full_content();
                            let tool_calls = if parser.has_tool_calls() {
                                Some(parser.finalize_tool_calls())
                            } else {
                                None
                            };
                            let usage = parser.get_usage();

                            if send_done {
                                let _ = tx
                                    .send(StreamEvent::Done {
                                        usage: usage.clone(),
                                    })
                                    .await;
                            }

                            return Ok(StreamResult {
                                content: full_content,
                                tool_calls,
                                usage,
                                reasoning_content: None,
                            });
                        }
                    }
                }
                Err(e) => {
                    error!("[AnthropicProtocol] 流读取错误: {}", e);
                    let _ = tx
                        .send(StreamEvent::Error {
                            message: format!("流读取错误: {}", e),
                        })
                        .await;
                    return Err(format!("流读取错误: {}", e));
                }
            }
        }

        // 流正常结束
        let full_content = parser.get_full_content();
        let tool_calls = if parser.has_tool_calls() {
            Some(parser.finalize_tool_calls())
        } else {
            None
        };
        let usage = parser.get_usage();

        if send_done {
            let _ = tx
                .send(StreamEvent::Done {
                    usage: usage.clone(),
                })
                .await;
        }

        Ok(StreamResult {
            content: full_content,
            tool_calls,
            usage,
            reasoning_content: None,
        })
    }
}

#[async_trait]
impl Protocol for AnthropicProtocol {
    async fn chat_stream(
        &self,
        client: &Client,
        base_url: &str,
        api_key: &str,
        messages: &[AgentMessage],
        user_message: &str,
        images: Option<&[ImageData]>,
        model: &str,
        config: &AgentConfig,
        tools: Option<&[Tool]>,
        tx: mpsc::Sender<StreamEvent>,
        provider_id: Option<&str>,
    ) -> Result<StreamResult, String> {
        info!(
            "[AnthropicProtocol] 发送流式请求: model={}, history_len={}, tools_count={}, provider_id={:?}",
            model,
            messages.len(),
            tools.map(|t| t.len()).unwrap_or(0),
            provider_id
        );

        let (anthropic_messages, system) =
            self.build_messages(messages, user_message, images, config);

        let anthropic_tools = Self::convert_tools(tools);

        let request = AnthropicMessagesRequest {
            model: model.to_string(),
            messages: anthropic_messages,
            max_tokens: config.max_tokens.unwrap_or(4096),
            stream: true,
            system,
            temperature: config.temperature,
            tools: anthropic_tools,
        };

        let url = format!("{}{}", base_url, self.endpoint());

        info!(
            "[AnthropicProtocol] 请求详情: url={}, model={}, messages_count={}, has_tools={}",
            url,
            model,
            request.messages.len(),
            request.tools.is_some()
        );

        let mut req_builder = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01");

        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!("[AnthropicProtocol] 请求失败: {} - {}", status, body);
            let _ = tx
                .send(StreamEvent::Error {
                    message: format!("API 错误 ({}): {}", status, body),
                })
                .await;
            return Err(format!("API 错误: {}", status));
        }

        Self::process_stream(response, tx, true).await
    }

    async fn chat_stream_continue(
        &self,
        client: &Client,
        base_url: &str,
        api_key: &str,
        messages: &[AgentMessage],
        model: &str,
        config: &AgentConfig,
        tools: Option<&[Tool]>,
        tx: mpsc::Sender<StreamEvent>,
        provider_id: Option<&str>,
    ) -> Result<StreamResult, String> {
        debug!(
            "[AnthropicProtocol] 继续流式对话: model={}, history_len={}, tools_count={}, provider_id={:?}",
            model,
            messages.len(),
            tools.map(|t| t.len()).unwrap_or(0),
            provider_id
        );

        let (anthropic_messages, system) = self.build_messages_from_history(messages, config);

        let anthropic_tools = Self::convert_tools(tools);

        let request = AnthropicMessagesRequest {
            model: model.to_string(),
            messages: anthropic_messages,
            max_tokens: config.max_tokens.unwrap_or(4096),
            stream: true,
            system,
            temperature: config.temperature,
            tools: anthropic_tools,
        };

        let url = format!("{}{}", base_url, self.endpoint());

        info!(
            "[AnthropicProtocol] 继续请求详情: url={}, model={}, messages_count={}, has_tools={}",
            url,
            model,
            request.messages.len(),
            request.tools.is_some()
        );

        // 调试：打印消息序列
        for (i, msg) in request.messages.iter().enumerate() {
            debug!(
                "[AnthropicProtocol] Message {}: role={}, content_type={}",
                i,
                msg.role,
                if msg.content.is_string() {
                    "string"
                } else {
                    "array"
                }
            );
            if let Some(content_array) = msg.content.as_array() {
                for (j, block) in content_array.iter().enumerate() {
                    if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                        debug!(
                            "[AnthropicProtocol] Message {} Block {}: type={}",
                            i, j, block_type
                        );
                        if block_type == "tool_result" {
                            if let Some(tool_use_id) =
                                block.get("tool_use_id").and_then(|id| id.as_str())
                            {
                                debug!(
                                    "[AnthropicProtocol] tool_result for tool_use_id: {}",
                                    tool_use_id
                                );
                            }
                        } else if block_type == "tool_use" {
                            if let Some(tool_id) = block.get("id").and_then(|id| id.as_str()) {
                                debug!("[AnthropicProtocol] tool_use with id: {}", tool_id);
                            }
                        }
                    }
                }
            }
        }

        let mut req_builder = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01");

        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!("[AnthropicProtocol] 请求失败: {} - {}", status, body);
            let _ = tx
                .send(StreamEvent::Error {
                    message: format!("API 错误 ({}): {}", status, body),
                })
                .await;
            return Err(format!("API 错误: {}", status));
        }

        // 继续对话时不发送 Done 事件
        Self::process_stream(response, tx, false).await
    }

    fn endpoint(&self) -> &'static str {
        "/v1/messages"
    }
}
