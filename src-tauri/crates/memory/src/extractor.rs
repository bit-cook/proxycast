//! LLM-assisted memory extraction
//!
//! Uses Claude/GPT to extract high-quality memories from conversations

use crate::gatekeeper::ChatMessage;
use crate::models::{MemoryCategory, MemoryMetadata, MemorySource, MemoryType, UnifiedMemory};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ==================== Types ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub title: String,
    pub category: MemoryCategory,
    pub summary: String,
    pub content: String,
    pub importance: u8,
    pub tags: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct ExtractionContext {
    pub messages: Vec<ChatMessage>,
    pub existing_memories: Vec<UnifiedMemory>,
    pub session_id: String,
}

// ==================== Prompt Building ====================

pub fn build_extraction_prompt(context: &ExtractionContext) -> String {
    let existing_summary = if context.existing_memories.is_empty() {
        "无已有记忆".to_string()
    } else {
        context
            .existing_memories
            .iter()
            .take(5)
            .map(|m| format!("- {}: {}", m.title, m.summary))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let messages_text = context
        .messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"你是记忆提取专家。分析对话，提取重要的用户信息。

## 已有记忆
{existing_summary}

## 新对话
{messages_text}

## 提取规则
1. 只提取**新的、重要**的信息
2. 避免重复已有记忆
3. 分类准确：
   - identity: 姓名、联系方式、个人特征
   - context: 工作、学习环境、背景
   - preference: 喜好、习惯、爱好
   - experience: 技能、经历、成就
   - activity: 会议、任务、事件
4. 标题简洁（10字内）
5. 摘要精炼（一句话）

## 输出格式
```json
[
    {{
        "title": "简短标题",
        "category": "identity|context|preference|experience|activity",
        "summary": "一句话摘要",
        "content": "详细内容（2-3句话）",
        "importance": 1-10,
        "tags": ["标签1", "标签2"],
        "confidence": 0.0-1.0
    }}
]
```

只提取真正重要的信息。如果没有新信息，返回空数组 []。"#
    )
}

// ==================== LLM API ====================

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

pub async fn call_claude_api(api_key: &str, prompt: &str, model: &str) -> Result<String, String> {
    let client = Client::new();
    let url = "https://api.anthropic.com/v1/messages";

    let request = ClaudeRequest {
        model: model.to_string(),
        max_tokens: 2048,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {status}: {body}"));
    }

    let body: ClaudeResponse = response
        .json()
        .await
        .map_err(|e| format!("JSON parse failed: {e}"))?;

    Ok(body
        .content
        .first()
        .map(|c| c.text.clone())
        .unwrap_or_default())
}

// ==================== Extraction ====================

pub async fn extract_memories(
    api_key: &str,
    context: &ExtractionContext,
) -> Result<Vec<UnifiedMemory>, String> {
    let prompt = build_extraction_prompt(context);

    let response = call_claude_api(api_key, &prompt, "claude-3-5-sonnet-20241022").await?;

    let extracted = parse_extraction_response(&response)?;
    let validated = validate_memories(extracted)?;

    let memories = validated
        .into_iter()
        .map(|e| convert_to_unified_memory(e, &context.session_id))
        .collect();

    Ok(memories)
}

fn parse_extraction_response(response: &str) -> Result<Vec<ExtractedMemory>, String> {
    let json_start = response.find('[').ok_or("No JSON array found")?;
    let json_end = response.rfind(']').ok_or("No JSON array end found")?;
    let json_str = &response[json_start..=json_end];

    serde_json::from_str(json_str).map_err(|e| format!("JSON parse failed: {e}"))
}

fn validate_memories(memories: Vec<ExtractedMemory>) -> Result<Vec<ExtractedMemory>, String> {
    let validated: Vec<_> = memories
        .into_iter()
        .filter(|m| {
            !m.title.is_empty() && m.title.len() <= 50 && m.importance >= 3 && m.confidence >= 0.5
        })
        .collect();

    Ok(validated)
}

fn convert_to_unified_memory(extracted: ExtractedMemory, session_id: &str) -> UnifiedMemory {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    UnifiedMemory {
        id: format!("mem_{now}"),
        session_id: session_id.to_string(),
        memory_type: MemoryType::Conversation,
        category: extracted.category,
        title: extracted.title,
        content: extracted.content,
        summary: extracted.summary,
        tags: extracted.tags,
        metadata: MemoryMetadata {
            confidence: extracted.confidence,
            importance: extracted.importance,
            access_count: 0,
            last_accessed_at: None,
            source: MemorySource::AutoExtracted,
            embedding: None,
        },
        created_at: now,
        updated_at: now,
        archived: false,
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extraction_response() {
        let response = r#"Here are the memories:
```json
[
    {
        "title": "喜欢咖啡",
        "category": "preference",
        "summary": "用户喜欢喝咖啡",
        "content": "用户表示喜欢喝咖啡，特别是美式咖啡",
        "importance": 5,
        "tags": ["咖啡", "饮品"],
        "confidence": 0.9
    }
]
```"#;

        let result = parse_extraction_response(response);
        assert!(result.is_ok());
        let memories = result.unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].title, "喜欢咖啡");
    }

    #[test]
    fn test_validate_memories() {
        let memories = vec![
            ExtractedMemory {
                title: "Valid".to_string(),
                category: MemoryCategory::Preference,
                summary: "Summary".to_string(),
                content: "Content".to_string(),
                importance: 5,
                tags: vec![],
                confidence: 0.8,
            },
            ExtractedMemory {
                title: "".to_string(), // Invalid: empty title
                category: MemoryCategory::Preference,
                summary: "Summary".to_string(),
                content: "Content".to_string(),
                importance: 5,
                tags: vec![],
                confidence: 0.8,
            },
            ExtractedMemory {
                title: "Low importance".to_string(),
                category: MemoryCategory::Preference,
                summary: "Summary".to_string(),
                content: "Content".to_string(),
                importance: 2, // Invalid: too low
                tags: vec![],
                confidence: 0.8,
            },
        ];

        let result = validate_memories(memories).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Valid");
    }
}
