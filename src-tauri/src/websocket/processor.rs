//! WebSocket 消息处理器
//!
//! 解析 WebSocket 消息为 API 请求并复用现有请求处理逻辑

use super::{
    WsApiRequest, WsApiResponse, WsEndpoint, WsError, WsMessage, WsStreamChunk, WsStreamEnd,
};
use crate::models::anthropic::AnthropicMessagesRequest;
use crate::models::openai::ChatCompletionRequest;
use serde_json::Value;

/// 消息处理器
pub struct MessageProcessor;

impl MessageProcessor {
    /// 解析 ChatCompletions 请求
    pub fn parse_chat_completions(payload: &Value) -> Result<ChatCompletionRequest, WsError> {
        serde_json::from_value(payload.clone()).map_err(|e| {
            WsError::invalid_request(
                None,
                format!("Failed to parse ChatCompletionRequest: {}", e),
            )
        })
    }

    /// 解析 Anthropic Messages 请求
    pub fn parse_messages(payload: &Value) -> Result<AnthropicMessagesRequest, WsError> {
        serde_json::from_value(payload.clone()).map_err(|e| {
            WsError::invalid_request(
                None,
                format!("Failed to parse AnthropicMessagesRequest: {}", e),
            )
        })
    }

    /// 验证请求 payload
    pub fn validate_request(request: &WsApiRequest) -> Result<(), WsError> {
        // 验证 request_id 不为空
        if request.request_id.is_empty() {
            return Err(WsError::invalid_request(None, "request_id cannot be empty"));
        }

        // 验证 payload 是对象
        if !request.payload.is_object() {
            return Err(WsError::invalid_request(
                Some(request.request_id.clone()),
                "payload must be a JSON object",
            ));
        }

        // 根据端点类型验证必需字段
        match request.endpoint {
            WsEndpoint::ChatCompletions => {
                Self::validate_chat_completions_payload(&request.request_id, &request.payload)?;
            }
            WsEndpoint::Messages => {
                Self::validate_messages_payload(&request.request_id, &request.payload)?;
            }
            WsEndpoint::Models => {
                // Models 端点不需要特殊验证
            }
        }

        Ok(())
    }

    /// 验证 ChatCompletions payload
    fn validate_chat_completions_payload(request_id: &str, payload: &Value) -> Result<(), WsError> {
        let obj = payload.as_object().ok_or_else(|| {
            WsError::invalid_request(Some(request_id.to_string()), "payload must be an object")
        })?;

        // 验证 model 字段
        if !obj.contains_key("model") {
            return Err(WsError::invalid_request(
                Some(request_id.to_string()),
                "missing required field: model",
            ));
        }

        // 验证 messages 字段
        if !obj.contains_key("messages") {
            return Err(WsError::invalid_request(
                Some(request_id.to_string()),
                "missing required field: messages",
            ));
        }

        let messages = obj.get("messages").and_then(|v| v.as_array());
        if messages.is_none() || messages.unwrap().is_empty() {
            return Err(WsError::invalid_request(
                Some(request_id.to_string()),
                "messages must be a non-empty array",
            ));
        }

        Ok(())
    }

    /// 验证 Messages payload (Anthropic 格式)
    fn validate_messages_payload(request_id: &str, payload: &Value) -> Result<(), WsError> {
        let obj = payload.as_object().ok_or_else(|| {
            WsError::invalid_request(Some(request_id.to_string()), "payload must be an object")
        })?;

        // 验证 model 字段
        if !obj.contains_key("model") {
            return Err(WsError::invalid_request(
                Some(request_id.to_string()),
                "missing required field: model",
            ));
        }

        // 验证 messages 字段
        if !obj.contains_key("messages") {
            return Err(WsError::invalid_request(
                Some(request_id.to_string()),
                "missing required field: messages",
            ));
        }

        // 验证 max_tokens 字段 (Anthropic 要求)
        if !obj.contains_key("max_tokens") {
            return Err(WsError::invalid_request(
                Some(request_id.to_string()),
                "missing required field: max_tokens",
            ));
        }

        Ok(())
    }

    /// 创建成功响应
    pub fn create_response(request_id: &str, payload: Value) -> WsMessage {
        WsMessage::Response(WsApiResponse {
            request_id: request_id.to_string(),
            payload,
        })
    }

    /// 创建错误响应
    pub fn create_error_response(request_id: Option<&str>, error: WsError) -> WsMessage {
        WsMessage::Error(WsError {
            request_id: request_id.map(|s| s.to_string()),
            ..error
        })
    }

    /// 创建流式响应块
    pub fn create_stream_chunk(request_id: &str, index: u32, data: &str) -> WsMessage {
        WsMessage::StreamChunk(WsStreamChunk {
            request_id: request_id.to_string(),
            index,
            data: data.to_string(),
        })
    }

    /// 创建流式响应结束消息
    pub fn create_stream_end(request_id: &str, total_chunks: u32) -> WsMessage {
        WsMessage::StreamEnd(WsStreamEnd {
            request_id: request_id.to_string(),
            total_chunks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_request_empty_request_id() {
        let request = WsApiRequest {
            request_id: "".to_string(),
            endpoint: WsEndpoint::Models,
            payload: serde_json::json!({}),
        };
        let result = MessageProcessor::validate_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_non_object_payload() {
        let request = WsApiRequest {
            request_id: "req-1".to_string(),
            endpoint: WsEndpoint::Models,
            payload: serde_json::json!("not an object"),
        };
        let result = MessageProcessor::validate_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_chat_completions_missing_model() {
        let request = WsApiRequest {
            request_id: "req-1".to_string(),
            endpoint: WsEndpoint::ChatCompletions,
            payload: serde_json::json!({
                "messages": [{"role": "user", "content": "hello"}]
            }),
        };
        let result = MessageProcessor::validate_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_chat_completions_missing_messages() {
        let request = WsApiRequest {
            request_id: "req-1".to_string(),
            endpoint: WsEndpoint::ChatCompletions,
            payload: serde_json::json!({
                "model": "gpt-4"
            }),
        };
        let result = MessageProcessor::validate_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_chat_completions_valid() {
        let request = WsApiRequest {
            request_id: "req-1".to_string(),
            endpoint: WsEndpoint::ChatCompletions,
            payload: serde_json::json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "hello"}]
            }),
        };
        let result = MessageProcessor::validate_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_messages_missing_max_tokens() {
        let request = WsApiRequest {
            request_id: "req-1".to_string(),
            endpoint: WsEndpoint::Messages,
            payload: serde_json::json!({
                "model": "claude-3",
                "messages": [{"role": "user", "content": "hello"}]
            }),
        };
        let result = MessageProcessor::validate_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_messages_valid() {
        let request = WsApiRequest {
            request_id: "req-1".to_string(),
            endpoint: WsEndpoint::Messages,
            payload: serde_json::json!({
                "model": "claude-3",
                "messages": [{"role": "user", "content": "hello"}],
                "max_tokens": 1024
            }),
        };
        let result = MessageProcessor::validate_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_models_endpoint() {
        let request = WsApiRequest {
            request_id: "req-1".to_string(),
            endpoint: WsEndpoint::Models,
            payload: serde_json::json!({}),
        };
        let result = MessageProcessor::validate_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_response() {
        let response =
            MessageProcessor::create_response("req-1", serde_json::json!({"result": "success"}));
        match response {
            WsMessage::Response(resp) => {
                assert_eq!(resp.request_id, "req-1");
                assert_eq!(resp.payload["result"], "success");
            }
            _ => panic!("Expected Response message"),
        }
    }

    #[test]
    fn test_create_stream_chunk() {
        let chunk = MessageProcessor::create_stream_chunk("req-1", 5, "data: test");
        match chunk {
            WsMessage::StreamChunk(c) => {
                assert_eq!(c.request_id, "req-1");
                assert_eq!(c.index, 5);
                assert_eq!(c.data, "data: test");
            }
            _ => panic!("Expected StreamChunk message"),
        }
    }

    #[test]
    fn test_create_stream_end() {
        let end = MessageProcessor::create_stream_end("req-1", 10);
        match end {
            WsMessage::StreamEnd(e) => {
                assert_eq!(e.request_id, "req-1");
                assert_eq!(e.total_chunks, 10);
            }
            _ => panic!("Expected StreamEnd message"),
        }
    }

    #[test]
    fn test_parse_chat_completions() {
        let payload = serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false
        });
        let result = MessageProcessor::parse_chat_completions(&payload);
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
    }
}
