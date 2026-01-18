//! OpenAI SSE 流解析器
//!
//! 解析 OpenAI 兼容 API 的 Server-Sent Events 流
//! Requirements: 1.1, 1.3, 1.4

use crate::agent::types::{FunctionCall, TokenUsage, ToolCall};
use serde_json::Value;
use std::collections::HashMap;
use tracing::warn;

/// 工具调用增量数据
#[derive(Debug, Clone, Default)]
struct ToolCallDelta {
    /// 工具调用索引
    #[allow(dead_code)]
    index: usize,
    /// 工具调用 ID
    id: String,
    /// 工具类型
    call_type: String,
    /// 函数名
    function_name: String,
    /// 函数参数（累积的 JSON 字符串）
    function_arguments: String,
}

/// OpenAI SSE 流解析器
///
/// 解析 Server-Sent Events 流，提取 text_delta 和 tool_calls
#[derive(Debug, Default)]
pub struct OpenAISSEParser {
    /// 累积的完整内容
    full_content: String,
    /// 累积的推理内容（DeepSeek R1 等模型）
    reasoning_content: String,
    /// 当前正在构建的工具调用索引
    current_tool_indices: HashMap<usize, ToolCallDelta>,
}

impl OpenAISSEParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// 解析 SSE 数据行
    ///
    /// 返回 (text_delta, reasoning_delta, is_done, usage)
    /// - text_delta: 普通文本内容增量
    /// - reasoning_delta: 推理内容增量（DeepSeek reasoner 等模型）
    pub fn parse_data(
        &mut self,
        data: &str,
    ) -> (Option<String>, Option<String>, bool, Option<TokenUsage>) {
        if data.trim() == "[DONE]" {
            return (None, None, true, None);
        }

        let json: Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(e) => {
                warn!("[OpenAISSEParser] 解析 JSON 失败: {} - data: {}", e, data);
                return (None, None, false, None);
            }
        };

        // 提取 usage 信息（如果存在）
        let usage = json.get("usage").and_then(|u| {
            let input = u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let output = u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            if input > 0 || output > 0 {
                Some(TokenUsage::new(input, output))
            } else {
                None
            }
        });

        // 检查是否有 choices
        let choices = match json.get("choices").and_then(|c| c.as_array()) {
            Some(c) => c,
            None => return (None, None, false, usage),
        };

        if choices.is_empty() {
            return (None, None, false, usage);
        }

        let choice = &choices[0];
        let delta = match choice.get("delta") {
            Some(d) => d,
            None => return (None, None, false, usage),
        };

        // 检查 finish_reason
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|f| f.as_str())
            .unwrap_or("");
        let is_done = finish_reason == "stop" || finish_reason == "tool_calls";

        // 提取文本内容
        let text_delta = delta
            .get("content")
            .and_then(|c| c.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| {
                self.full_content.push_str(s);
                s.to_string()
            });

        // 提取推理内容（DeepSeek reasoner 等模型）
        let reasoning_delta = delta
            .get("reasoning_content")
            .and_then(|c| c.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| {
                self.reasoning_content.push_str(s);
                s.to_string()
            });

        // 提取工具调用
        if let Some(tool_calls) = delta.get("tool_calls").and_then(|tc| tc.as_array()) {
            for tc in tool_calls {
                self.parse_tool_call_delta(tc);
            }
        }

        (text_delta, reasoning_delta, is_done, usage)
    }

    /// 解析工具调用增量
    fn parse_tool_call_delta(&mut self, tc: &Value) {
        let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;

        // 获取或创建工具调用
        let tool_call = self
            .current_tool_indices
            .entry(index)
            .or_insert_with(|| ToolCallDelta {
                index,
                ..Default::default()
            });

        // 更新 ID
        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
            tool_call.id = id.to_string();
        }

        // 更新类型
        if let Some(t) = tc.get("type").and_then(|t| t.as_str()) {
            tool_call.call_type = t.to_string();
        }

        // 更新函数信息
        if let Some(function) = tc.get("function") {
            if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                tool_call.function_name = name.to_string();
            }
            if let Some(args) = function.get("arguments").and_then(|a| a.as_str()) {
                tool_call.function_arguments.push_str(args);
            }
        }
    }

    /// 完成解析，返回最终的工具调用列表
    pub fn finalize_tool_calls(&mut self) -> Vec<ToolCall> {
        // 按索引排序并转换为 ToolCall
        let mut indices: Vec<_> = self.current_tool_indices.keys().cloned().collect();
        indices.sort();

        indices
            .into_iter()
            .filter_map(|idx| {
                let delta = self.current_tool_indices.get(&idx)?;
                if delta.id.is_empty() || delta.function_name.is_empty() {
                    return None;
                }
                Some(ToolCall {
                    id: delta.id.clone(),
                    call_type: if delta.call_type.is_empty() {
                        "function".to_string()
                    } else {
                        delta.call_type.clone()
                    },
                    function: FunctionCall {
                        name: delta.function_name.clone(),
                        arguments: delta.function_arguments.clone(),
                    },
                })
            })
            .collect()
    }

    /// 获取完整内容
    pub fn get_full_content(&self) -> String {
        self.full_content.clone()
    }

    /// 获取推理内容（DeepSeek R1 等模型）
    pub fn get_reasoning_content(&self) -> Option<String> {
        if self.reasoning_content.is_empty() {
            None
        } else {
            Some(self.reasoning_content.clone())
        }
    }

    /// 是否有工具调用
    pub fn has_tool_calls(&self) -> bool {
        !self.current_tool_indices.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_delta() {
        let mut parser = OpenAISSEParser::new();

        let data1 = r#"{"choices":[{"delta":{"content":"Hello"}}]}"#;
        let data2 = r#"{"choices":[{"delta":{"content":" World"}}]}"#;
        let data3 = r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#;

        let (text1, _, done1, _) = parser.parse_data(data1);
        assert_eq!(text1, Some("Hello".to_string()));
        assert!(!done1);

        let (text2, _, done2, _) = parser.parse_data(data2);
        assert_eq!(text2, Some(" World".to_string()));
        assert!(!done2);

        let (text3, _, done3, _) = parser.parse_data(data3);
        assert!(text3.is_none());
        assert!(done3);

        assert_eq!(parser.get_full_content(), "Hello World");
    }

    #[test]
    fn test_reasoning_content() {
        let mut parser = OpenAISSEParser::new();

        let data1 = r#"{"choices":[{"delta":{"reasoning_content":"Let me think"}}]}"#;
        let data2 = r#"{"choices":[{"delta":{"reasoning_content":" about this"}}]}"#;
        let data3 = r#"{"choices":[{"delta":{"content":"The answer is 42"}}]}"#;

        let (text1, reasoning1, done1, _) = parser.parse_data(data1);
        assert!(text1.is_none());
        assert_eq!(reasoning1, Some("Let me think".to_string()));
        assert!(!done1);

        let (text2, reasoning2, done2, _) = parser.parse_data(data2);
        assert!(text2.is_none());
        assert_eq!(reasoning2, Some(" about this".to_string()));
        assert!(!done2);

        let (text3, reasoning3, done3, _) = parser.parse_data(data3);
        assert_eq!(text3, Some("The answer is 42".to_string()));
        assert!(reasoning3.is_none());
        assert!(!done3);

        assert_eq!(parser.get_full_content(), "The answer is 42");
        assert_eq!(
            parser.get_reasoning_content(),
            Some("Let me think about this".to_string())
        );
    }

    #[test]
    fn test_tool_calls() {
        let mut parser = OpenAISSEParser::new();

        let data1 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","type":"function","function":{"name":"bash"}}]}}]}"#;
        let data2 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"command\":"}}]}}]}"#;
        let data3 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"ls -la\"}"}}]}}]}"#;
        let data4 = r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#;

        parser.parse_data(data1);
        parser.parse_data(data2);
        parser.parse_data(data3);
        let (_, _, done, _) = parser.parse_data(data4);

        assert!(done);
        assert!(parser.has_tool_calls());

        let tool_calls = parser.finalize_tool_calls();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_123");
        assert_eq!(tool_calls[0].function.name, "bash");
        assert_eq!(tool_calls[0].function.arguments, r#"{"command":"ls -la"}"#);
    }

    #[test]
    fn test_usage() {
        let mut parser = OpenAISSEParser::new();

        let data = r#"{"choices":[{"delta":{"content":"Hi"}}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
        let (text, _, _, usage) = parser.parse_data(data);

        assert_eq!(text, Some("Hi".to_string()));
        assert!(usage.is_some());
        let usage = usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
    }

    #[test]
    fn test_done_signal() {
        let mut parser = OpenAISSEParser::new();

        let (_, _, done, _) = parser.parse_data("[DONE]");
        assert!(done);
    }
}
