//! OpenAI SSE 生成器
//!
//! 将 `StreamEvent` 转换为 OpenAI Chat Completions SSE 格式。
//!
//! # 格式说明
//!
//! OpenAI SSE 格式：
//! ```text
//! data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}
//!
//! data: [DONE]
//! ```

use crate::stream::events::{ContentBlockType, StopReason, StreamEvent};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// OpenAI SSE 生成器
#[derive(Debug)]
pub struct OpenAiSseGenerator {
    /// 响应 ID
    response_id: String,
    /// 模型名称
    model: String,
    /// 创建时间戳
    created: u64,
    /// 工具调用状态 (tool_call_id -> (index, name, accumulated_args))
    tool_calls: HashMap<String, ToolCallState>,
    /// 下一个工具调用索引
    next_tool_index: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ToolCallState {
    /// 工具调用在 tool_calls 数组中的索引
    index: usize,
    /// 工具名称
    name: String,
    /// 累积的参数
    arguments: String,
}

impl Default for OpenAiSseGenerator {
    fn default() -> Self {
        Self::new("unknown".to_string())
    }
}

impl OpenAiSseGenerator {
    /// 创建新的生成器
    pub fn new(model: String) -> Self {
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            response_id: format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
            model,
            created,
            tool_calls: HashMap::new(),
            next_tool_index: 0,
        }
    }

    /// 使用指定的响应 ID 创建生成器
    pub fn with_id(id: String, model: String) -> Self {
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            response_id: id,
            model,
            created,
            tool_calls: HashMap::new(),
            next_tool_index: 0,
        }
    }

    /// 将 StreamEvent 转换为 OpenAI SSE 字符串
    ///
    /// # 返回
    ///
    /// - `Some(String)` - 生成的 SSE 字符串（包含 `data: ` 前缀和换行）
    /// - `None` - 该事件不需要生成 SSE 输出
    pub fn generate(&mut self, event: &StreamEvent) -> Option<String> {
        match event {
            StreamEvent::MessageStart { id, model } => {
                self.response_id = id.clone();
                self.model = model.clone();
                // OpenAI 格式不需要单独的 message_start 事件
                None
            }

            StreamEvent::ContentBlockStart { block_type, .. } => {
                // OpenAI 格式不需要单独的 content_block_start 事件
                // 但我们需要跟踪工具调用
                if let ContentBlockType::ToolUse { id, name } = block_type {
                    let index = self.next_tool_index;
                    self.next_tool_index += 1;
                    self.tool_calls.insert(
                        id.clone(),
                        ToolCallState {
                            index,
                            name: name.clone(),
                            arguments: String::new(),
                        },
                    );
                }
                None
            }

            StreamEvent::TextDelta { text } => {
                let chunk = OpenAiStreamChunk {
                    id: &self.response_id,
                    object: "chat.completion.chunk",
                    created: self.created,
                    model: &self.model,
                    choices: vec![OpenAiChoice {
                        index: 0,
                        delta: OpenAiDelta {
                            role: None,
                            content: Some(text.as_str()),
                            tool_calls: None,
                        },
                        finish_reason: None,
                    }],
                };
                Some(format!("data: {}\n\n", serde_json::to_string(&chunk).ok()?))
            }

            StreamEvent::ToolUseStart { id, name } => {
                // 确保工具调用状态存在
                let index = if let Some(state) = self.tool_calls.get(id) {
                    state.index
                } else {
                    let index = self.next_tool_index;
                    self.next_tool_index += 1;
                    self.tool_calls.insert(
                        id.clone(),
                        ToolCallState {
                            index,
                            name: name.clone(),
                            arguments: String::new(),
                        },
                    );
                    index
                };

                let chunk = OpenAiStreamChunk {
                    id: &self.response_id,
                    object: "chat.completion.chunk",
                    created: self.created,
                    model: &self.model,
                    choices: vec![OpenAiChoice {
                        index: 0,
                        delta: OpenAiDelta {
                            role: None,
                            content: None,
                            tool_calls: Some(vec![OpenAiToolCallDelta {
                                index,
                                id: Some(id.as_str()),
                                r#type: Some("function"),
                                function: Some(OpenAiFunctionDelta {
                                    name: Some(name.as_str()),
                                    arguments: None,
                                }),
                            }]),
                        },
                        finish_reason: None,
                    }],
                };
                Some(format!("data: {}\n\n", serde_json::to_string(&chunk).ok()?))
            }

            StreamEvent::ToolUseInputDelta { id, partial_json } => {
                let index = self.tool_calls.get(id)?.index;

                // 累积参数
                if let Some(state) = self.tool_calls.get_mut(id) {
                    state.arguments.push_str(partial_json);
                }

                let chunk = OpenAiStreamChunk {
                    id: &self.response_id,
                    object: "chat.completion.chunk",
                    created: self.created,
                    model: &self.model,
                    choices: vec![OpenAiChoice {
                        index: 0,
                        delta: OpenAiDelta {
                            role: None,
                            content: None,
                            tool_calls: Some(vec![OpenAiToolCallDelta {
                                index,
                                id: None,
                                r#type: None,
                                function: Some(OpenAiFunctionDelta {
                                    name: None,
                                    arguments: Some(partial_json.as_str()),
                                }),
                            }]),
                        },
                        finish_reason: None,
                    }],
                };
                Some(format!("data: {}\n\n", serde_json::to_string(&chunk).ok()?))
            }

            StreamEvent::ToolUseStop { id } => {
                // OpenAI 格式不需要单独的工具调用结束事件
                self.tool_calls.remove(id);
                None
            }

            StreamEvent::ContentBlockStop { .. } => {
                // OpenAI 格式不需要单独的 content_block_stop 事件
                None
            }

            StreamEvent::MessageStop { stop_reason } => {
                let finish_reason = stop_reason.to_openai_str();

                let chunk = OpenAiStreamChunk {
                    id: &self.response_id,
                    object: "chat.completion.chunk",
                    created: self.created,
                    model: &self.model,
                    choices: vec![OpenAiChoice {
                        index: 0,
                        delta: OpenAiDelta {
                            role: None,
                            content: None,
                            tool_calls: None,
                        },
                        finish_reason: Some(finish_reason),
                    }],
                };

                let chunk_str = format!("data: {}\n\n", serde_json::to_string(&chunk).ok()?);
                Some(format!("{}data: [DONE]\n\n", chunk_str))
            }

            StreamEvent::Usage {
                input_tokens,
                output_tokens,
                ..
            } => {
                // OpenAI 在流式响应中通常不发送 usage
                // 但某些实现可能需要，这里可以选择性地生成
                let _ = (input_tokens, output_tokens);
                None
            }

            StreamEvent::BackendUsage { .. } => {
                // 后端特定的使用量信息，不转换为 OpenAI 格式
                None
            }

            StreamEvent::Error {
                error_type,
                message,
            } => {
                // 生成错误响应
                let error_obj = serde_json::json!({
                    "error": {
                        "type": error_type,
                        "message": message,
                    }
                });
                Some(format!("data: {}\n\n", error_obj))
            }

            StreamEvent::Ping => {
                // 心跳事件，生成空的 SSE 注释
                Some(": ping\n\n".to_string())
            }
        }
    }

    /// 生成 [DONE] 事件
    pub fn generate_done(&self) -> String {
        "data: [DONE]\n\n".to_string()
    }

    /// 获取响应 ID
    pub fn response_id(&self) -> &str {
        &self.response_id
    }
}

// ============================================================================
// OpenAI SSE 数据结构
// ============================================================================

#[derive(Debug, Serialize)]
struct OpenAiStreamChunk<'a> {
    id: &'a str,
    object: &'a str,
    created: u64,
    model: &'a str,
    choices: Vec<OpenAiChoice<'a>>,
}

#[derive(Debug, Serialize)]
struct OpenAiChoice<'a> {
    index: usize,
    delta: OpenAiDelta<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct OpenAiDelta<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCallDelta<'a>>>,
}

#[derive(Debug, Serialize)]
struct OpenAiToolCallDelta<'a> {
    index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function: Option<OpenAiFunctionDelta<'a>>,
}

#[derive(Debug, Serialize)]
struct OpenAiFunctionDelta<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_text_delta() {
        let mut generator = OpenAiSseGenerator::new("gpt-4".to_string());
        let event = StreamEvent::TextDelta {
            text: "Hello".to_string(),
        };

        let sse = generator.generate(&event);
        assert!(sse.is_some());
        let sse = sse.unwrap();
        assert!(sse.starts_with("data: "));
        assert!(sse.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_generate_tool_call() {
        let mut generator = OpenAiSseGenerator::new("gpt-4".to_string());

        // 工具调用开始
        let event = StreamEvent::ToolUseStart {
            id: "call_123".to_string(),
            name: "read_file".to_string(),
        };
        let sse = generator.generate(&event);
        assert!(sse.is_some());
        let sse = sse.unwrap();
        assert!(sse.contains("\"tool_calls\""));
        assert!(sse.contains("\"name\":\"read_file\""));

        // 工具参数增量
        let event = StreamEvent::ToolUseInputDelta {
            id: "call_123".to_string(),
            partial_json: "{\"path\":".to_string(),
        };
        let sse = generator.generate(&event);
        assert!(sse.is_some());
        let sse = sse.unwrap();
        assert!(sse.contains("\"arguments\":\"{\\\"path\\\":\""));
    }

    #[test]
    fn test_generate_message_stop() {
        let mut generator = OpenAiSseGenerator::new("gpt-4".to_string());
        let event = StreamEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        };

        let sse = generator.generate(&event);
        assert!(sse.is_some());
        let sse = sse.unwrap();
        assert!(sse.contains("\"finish_reason\":\"stop\""));
        assert!(sse.contains("[DONE]"));
    }
}
