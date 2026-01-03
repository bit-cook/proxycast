//! Anthropic SSE 生成器
//!
//! 将 `StreamEvent` 转换为 Anthropic Messages API SSE 格式。
//!
//! # 格式说明
//!
//! Anthropic SSE 格式：
//! ```text
//! event: message_start
//! data: {"type":"message_start","message":{...}}
//!
//! event: content_block_start
//! data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}
//!
//! event: content_block_delta
//! data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}
//!
//! event: content_block_stop
//! data: {"type":"content_block_stop","index":0}
//!
//! event: message_delta
//! data: {"type":"message_delta","delta":{"stop_reason":"end_turn"}}
//!
//! event: message_stop
//! data: {"type":"message_stop"}
//! ```

use crate::stream::events::{ContentBlockType, StopReason, StreamEvent};
use std::collections::HashMap;
use uuid::Uuid;

/// 工具调用状态
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct ToolCallState {
    /// 工具调用 ID
    id: String,
    /// 工具名称
    name: String,
    /// 累积的输入 JSON
    input: String,
    /// 内容块索引
    index: u32,
}

/// Anthropic SSE 生成器
#[derive(Debug)]
pub struct AnthropicSseGenerator {
    /// 消息 ID
    message_id: String,
    /// 模型名称
    model: String,
    /// 是否已发送 message_start 事件
    message_started: bool,
    /// 工具调用状态映射
    tool_calls: HashMap<String, ToolCallState>,
    /// 输入 token 数量
    input_tokens: u32,
    /// 输出 token 数量
    output_tokens: u32,
    /// 缓存读取 token 数
    cache_read_input_tokens: u32,
    /// 缓存创建 token 数
    cache_creation_input_tokens: u32,
    /// 累积的停止原因
    stop_reason: Option<StopReason>,
}

impl Default for AnthropicSseGenerator {
    fn default() -> Self {
        Self::new("unknown".to_string())
    }
}

impl AnthropicSseGenerator {
    /// 创建新的生成器
    pub fn new(model: String) -> Self {
        Self {
            message_id: format!("msg_{}", Uuid::new_v4().simple()),
            model,
            message_started: false,
            tool_calls: HashMap::new(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            stop_reason: None,
        }
    }

    /// 使用指定的消息 ID 创建生成器
    pub fn with_id(id: String, model: String) -> Self {
        Self {
            message_id: id,
            model,
            message_started: false,
            tool_calls: HashMap::new(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            stop_reason: None,
        }
    }

    /// 将 StreamEvent 转换为 Anthropic SSE 字符串列表
    ///
    /// # 返回
    ///
    /// SSE 事件字符串列表，每个字符串都是完整的 SSE 事件（包含 `event:` 和 `data:` 行）
    pub fn generate(&mut self, event: &StreamEvent) -> Vec<String> {
        let mut sse_events = Vec::new();

        // 确保发送 message_start
        if !self.message_started {
            match event {
                StreamEvent::MessageStart { id, model } => {
                    self.message_id = id.clone();
                    self.model = model.clone();
                }
                _ => {}
            }
            sse_events.push(self.create_message_start());
            self.message_started = true;
        }

        match event {
            StreamEvent::MessageStart { .. } => {
                // 已经在上面处理了
            }

            StreamEvent::ContentBlockStart { index, block_type } => {
                match block_type {
                    ContentBlockType::Text => {
                        sse_events.push(self.create_content_block_start_text(*index));
                    }
                    ContentBlockType::ToolUse { id, name } => {
                        // 记录工具调用状态
                        self.tool_calls.insert(
                            id.clone(),
                            ToolCallState {
                                id: id.clone(),
                                name: name.clone(),
                                input: String::new(),
                                index: *index,
                            },
                        );
                        sse_events.push(self.create_content_block_start_tool(*index, id, name));
                    }
                }
            }

            StreamEvent::TextDelta { text } => {
                // 假设文本块索引为 0（大多数情况下）
                sse_events.push(self.create_text_delta(0, text));
            }

            StreamEvent::ToolUseStart { id, name } => {
                // 如果还没有对应的 ContentBlockStart，创建工具调用状态
                if !self.tool_calls.contains_key(id) {
                    let index = self.tool_calls.len() as u32;
                    self.tool_calls.insert(
                        id.clone(),
                        ToolCallState {
                            id: id.clone(),
                            name: name.clone(),
                            input: String::new(),
                            index,
                        },
                    );
                }
                // ToolUseStart 已经在 ContentBlockStart 中处理
            }

            StreamEvent::ToolUseInputDelta { id, partial_json } => {
                if let Some(state) = self.tool_calls.get_mut(id) {
                    state.input.push_str(partial_json);
                    let index = state.index;
                    sse_events.push(self.create_input_json_delta(index, partial_json));
                }
            }

            StreamEvent::ToolUseStop { id } => {
                // 工具调用结束，但保留状态直到 ContentBlockStop
                let _ = id;
            }

            StreamEvent::ContentBlockStop { index } => {
                sse_events.push(self.create_content_block_stop(*index));
            }

            StreamEvent::MessageStop { stop_reason } => {
                self.stop_reason = Some(stop_reason.clone());
                // 移除所有工具调用状态
                self.tool_calls.clear();
                sse_events.push(self.create_message_delta(stop_reason));
                sse_events.push(self.create_message_stop());
            }

            StreamEvent::Usage {
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
            } => {
                self.input_tokens = *input_tokens;
                self.output_tokens = *output_tokens;
                if let Some(cache_read) = cache_read_input_tokens {
                    self.cache_read_input_tokens = *cache_read;
                }
                if let Some(cache_creation) = cache_creation_input_tokens {
                    self.cache_creation_input_tokens = *cache_creation;
                }
                // Anthropic 在流中发送 ping 事件而不是 usage 事件
            }

            StreamEvent::BackendUsage { .. } => {
                // 后端特定的使用量信息，不转换
            }

            StreamEvent::Error {
                error_type,
                message,
            } => {
                sse_events.push(self.create_error(error_type, message));
            }

            StreamEvent::Ping => {
                sse_events.push(self.create_ping());
            }
        }

        sse_events
    }

    /// 获取消息 ID
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    /// 获取模型名称
    pub fn model(&self) -> &str {
        &self.model
    }

    // ========================================================================
    // SSE 事件创建方法
    // ========================================================================

    fn create_message_start(&self) -> String {
        let event = serde_json::json!({
            "type": "message_start",
            "message": {
                "id": self.message_id,
                "type": "message",
                "role": "assistant",
                "model": self.model,
                "content": [],
                "stop_reason": serde_json::Value::Null,
                "stop_sequence": serde_json::Value::Null,
                "usage": {
                    "input_tokens": self.input_tokens,
                    "output_tokens": self.output_tokens,
                    "cache_read_input_tokens": self.cache_read_input_tokens,
                    "cache_creation_input_tokens": self.cache_creation_input_tokens
                }
            }
        });
        format!("event: message_start\ndata: {}\n\n", event)
    }

    fn create_content_block_start_text(&self, index: u32) -> String {
        let event = serde_json::json!({
            "type": "content_block_start",
            "index": index,
            "content_block": {
                "type": "text",
                "text": ""
            }
        });
        format!("event: content_block_start\ndata: {}\n\n", event)
    }

    fn create_content_block_start_tool(&self, index: u32, id: &str, name: &str) -> String {
        let event = serde_json::json!({
            "type": "content_block_start",
            "index": index,
            "content_block": {
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": {}
            }
        });
        format!("event: content_block_start\ndata: {}\n\n", event)
    }

    fn create_text_delta(&self, index: u32, text: &str) -> String {
        let event = serde_json::json!({
            "type": "content_block_delta",
            "index": index,
            "delta": {
                "type": "text_delta",
                "text": text
            }
        });
        format!("event: content_block_delta\ndata: {}\n\n", event)
    }

    fn create_input_json_delta(&self, index: u32, partial_json: &str) -> String {
        let event = serde_json::json!({
            "type": "content_block_delta",
            "index": index,
            "delta": {
                "type": "input_json_delta",
                "partial_json": partial_json
            }
        });
        format!("event: content_block_delta\ndata: {}\n\n", event)
    }

    fn create_content_block_stop(&self, index: u32) -> String {
        let event = serde_json::json!({
            "type": "content_block_stop",
            "index": index
        });
        format!("event: content_block_stop\ndata: {}\n\n", event)
    }

    fn create_message_delta(&self, stop_reason: &StopReason) -> String {
        let event = serde_json::json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": stop_reason.to_anthropic_str(),
                "stop_sequence": serde_json::Value::Null
            },
            "usage": {
                "output_tokens": self.output_tokens
            }
        });
        format!("event: message_delta\ndata: {}\n\n", event)
    }

    fn create_message_stop(&self) -> String {
        let event = serde_json::json!({
            "type": "message_stop"
        });
        format!("event: message_stop\ndata: {}\n\n", event)
    }

    fn create_ping(&self) -> String {
        let event = serde_json::json!({
            "type": "ping"
        });
        format!("event: ping\ndata: {}\n\n", event)
    }

    fn create_error(&self, error_type: &str, message: &str) -> String {
        let event = serde_json::json!({
            "type": "error",
            "error": {
                "type": error_type,
                "message": message
            }
        });
        format!("event: error\ndata: {}\n\n", event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_message_start() {
        let mut generator = AnthropicSseGenerator::new("claude-3-sonnet".to_string());
        let event = StreamEvent::MessageStart {
            id: "msg_123".to_string(),
            model: "claude-3-sonnet".to_string(),
        };

        let sse = generator.generate(&event);
        assert_eq!(sse.len(), 1);
        assert!(sse[0].starts_with("event: message_start\ndata: "));
        assert!(sse[0].contains("\"id\":\"msg_123\""));
    }

    #[test]
    fn test_generate_text_content() {
        let mut generator = AnthropicSseGenerator::new("claude-3-sonnet".to_string());

        // 先发送 message_start
        let _ = generator.generate(&StreamEvent::MessageStart {
            id: "msg_123".to_string(),
            model: "claude-3-sonnet".to_string(),
        });

        // 内容块开始
        let sse = generator.generate(&StreamEvent::ContentBlockStart {
            index: 0,
            block_type: ContentBlockType::Text,
        });
        assert!(sse[0].contains("content_block_start"));
        assert!(sse[0].contains("\"type\":\"text\""));

        // 文本增量
        let sse = generator.generate(&StreamEvent::TextDelta {
            text: "Hello".to_string(),
        });
        assert!(sse[0].contains("content_block_delta"));
        assert!(sse[0].contains("text_delta"));
        assert!(sse[0].contains("Hello"));
    }

    #[test]
    fn test_generate_tool_use() {
        let mut generator = AnthropicSseGenerator::new("claude-3-sonnet".to_string());

        // 发送 message_start
        let _ = generator.generate(&StreamEvent::MessageStart {
            id: "msg_123".to_string(),
            model: "claude-3-sonnet".to_string(),
        });

        // 工具调用内容块开始
        let sse = generator.generate(&StreamEvent::ContentBlockStart {
            index: 1,
            block_type: ContentBlockType::ToolUse {
                id: "tool_abc".to_string(),
                name: "read_file".to_string(),
            },
        });
        assert!(sse[0].contains("content_block_start"));
        assert!(sse[0].contains("\"type\":\"tool_use\""));
        assert!(sse[0].contains("\"id\":\"tool_abc\""));
        assert!(sse[0].contains("\"name\":\"read_file\""));

        // 工具参数增量
        let sse = generator.generate(&StreamEvent::ToolUseInputDelta {
            id: "tool_abc".to_string(),
            partial_json: "{\"path\":".to_string(),
        });
        assert!(sse[0].contains("content_block_delta"));
        assert!(sse[0].contains("input_json_delta"));
    }

    #[test]
    fn test_generate_message_stop() {
        let mut generator = AnthropicSseGenerator::new("claude-3-sonnet".to_string());

        // 发送 message_start
        let _ = generator.generate(&StreamEvent::MessageStart {
            id: "msg_123".to_string(),
            model: "claude-3-sonnet".to_string(),
        });

        // 消息结束
        let sse = generator.generate(&StreamEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        });
        assert_eq!(sse.len(), 2);
        assert!(sse[0].contains("message_delta"));
        assert!(sse[0].contains("end_turn"));
        assert!(sse[1].contains("message_stop"));
    }
}
