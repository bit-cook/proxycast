//! 会话管理服务
//!
//! 提供会话的 CRUD 操作和消息管理功能
//!
//! ## 主要功能
//! - 会话创建、查询、删除、重命名
//! - 消息存储和检索
//! - 会话标题自动生成
//!
//! ## 依赖
//! - SQLite 数据库（通过 DatabaseService）
//! - types 模块中的数据结构

use super::types::{ChatMessage, ChatSession, ContentBlock, CreateMessageRequest, MessageRole};
use chrono::Utc;
use uuid::Uuid;

/// 会话管理服务
///
/// 负责管理通用对话的会话和消息
pub struct SessionService;

impl SessionService {
    /// 创建新会话
    ///
    /// # Arguments
    /// * `name` - 会话名称，如果为 None 则使用默认名称"新对话"
    ///
    /// # Returns
    /// 新创建的会话对象
    pub fn create_session(name: Option<String>) -> ChatSession {
        let now = Utc::now().timestamp_millis();
        ChatSession {
            id: Uuid::new_v4().to_string(),
            name: name.unwrap_or_else(|| "新对话".to_string()),
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }

    /// 创建新消息
    ///
    /// # Arguments
    /// * `request` - 创建消息请求
    ///
    /// # Returns
    /// 新创建的消息对象
    pub fn create_message(request: CreateMessageRequest) -> ChatMessage {
        let now = Utc::now().timestamp_millis();
        ChatMessage {
            id: Uuid::new_v4().to_string(),
            session_id: request.session_id,
            role: request.role,
            content: request.content,
            blocks: request.blocks,
            status: "complete".to_string(),
            created_at: now,
            metadata: request.metadata,
        }
    }

    /// 验证消息内容是否有效（非空白）
    ///
    /// # Arguments
    /// * `content` - 消息内容
    ///
    /// # Returns
    /// 如果内容有效返回 true，否则返回 false
    pub fn is_valid_message_content(content: &str) -> bool {
        !content.trim().is_empty()
    }

    /// 生成会话默认标题
    ///
    /// 根据第一条消息内容生成会话标题
    ///
    /// # Arguments
    /// * `first_message` - 第一条消息内容
    /// * `max_length` - 标题最大长度
    ///
    /// # Returns
    /// 生成的标题字符串
    pub fn generate_default_title(first_message: &str, max_length: usize) -> String {
        let trimmed = first_message.trim();
        if trimmed.is_empty() {
            return "新对话".to_string();
        }

        // 取第一行
        let first_line = trimmed.lines().next().unwrap_or(trimmed);

        // 截断到最大长度
        if first_line.chars().count() <= max_length {
            first_line.to_string()
        } else {
            let truncated: String = first_line.chars().take(max_length - 3).collect();
            format!("{}...", truncated)
        }
    }

    /// 解析消息内容中的代码块
    ///
    /// # Arguments
    /// * `content` - 消息内容
    ///
    /// # Returns
    /// 解析出的内容块列表
    pub fn parse_content_blocks(content: &str) -> Vec<ContentBlock> {
        let mut blocks = Vec::new();
        let mut current_pos = 0;

        // 简单的代码块解析：查找 ```language\n...\n```
        while let Some(start) = content[current_pos..].find("```") {
            let abs_start = current_pos + start;

            // 添加代码块之前的文本块
            if abs_start > current_pos {
                let text = &content[current_pos..abs_start];
                if !text.trim().is_empty() {
                    blocks.push(ContentBlock {
                        r#type: "text".to_string(),
                        content: text.to_string(),
                        language: None,
                        filename: None,
                        mime_type: None,
                    });
                }
            }

            // 查找代码块结束位置
            let code_start = abs_start + 3;
            if let Some(end) = content[code_start..].find("```") {
                let abs_end = code_start + end;
                let code_content = &content[code_start..abs_end];

                // 解析语言标识
                let (language, code) = if let Some(newline_pos) = code_content.find('\n') {
                    let lang = code_content[..newline_pos].trim();
                    let code = &code_content[newline_pos + 1..];
                    (
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang.to_string())
                        },
                        code.to_string(),
                    )
                } else {
                    (None, code_content.to_string())
                };

                blocks.push(ContentBlock {
                    r#type: "code".to_string(),
                    content: code,
                    language,
                    filename: None,
                    mime_type: None,
                });

                current_pos = abs_end + 3;
            } else {
                // 没有找到结束标记，将剩余内容作为文本
                break;
            }
        }

        // 添加剩余的文本
        if current_pos < content.len() {
            let remaining = &content[current_pos..];
            if !remaining.trim().is_empty() {
                blocks.push(ContentBlock {
                    r#type: "text".to_string(),
                    content: remaining.to_string(),
                    language: None,
                    filename: None,
                    mime_type: None,
                });
            }
        }

        blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session_with_name() {
        let session = SessionService::create_session(Some("测试会话".to_string()));
        assert_eq!(session.name, "测试会话");
        assert!(!session.id.is_empty());
        assert!(session.created_at > 0);
        assert_eq!(session.created_at, session.updated_at);
    }

    #[test]
    fn test_create_session_default_name() {
        let session = SessionService::create_session(None);
        assert_eq!(session.name, "新对话");
    }

    #[test]
    fn test_create_message() {
        let request = CreateMessageRequest {
            session_id: "session-1".to_string(),
            role: MessageRole::User,
            content: "你好".to_string(),
            blocks: None,
            metadata: None,
        };

        let message = SessionService::create_message(request);
        assert_eq!(message.session_id, "session-1");
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, "你好");
        assert_eq!(message.status, "complete");
    }

    #[test]
    fn test_is_valid_message_content() {
        assert!(SessionService::is_valid_message_content("hello"));
        assert!(SessionService::is_valid_message_content("  hello  "));
        assert!(!SessionService::is_valid_message_content(""));
        assert!(!SessionService::is_valid_message_content("   "));
        assert!(!SessionService::is_valid_message_content("\n\t"));
    }

    #[test]
    fn test_generate_default_title() {
        assert_eq!(
            SessionService::generate_default_title("你好，请帮我写一段代码", 20),
            "你好，请帮我写一段代码"
        );

        assert_eq!(
            SessionService::generate_default_title("这是一个非常非常非常长的标题需要被截断", 10),
            "这是一个非常非..."
        );

        assert_eq!(SessionService::generate_default_title("", 20), "新对话");

        assert_eq!(SessionService::generate_default_title("   ", 20), "新对话");

        // 多行内容取第一行
        assert_eq!(
            SessionService::generate_default_title("第一行\n第二行\n第三行", 20),
            "第一行"
        );
    }

    #[test]
    fn test_parse_content_blocks_simple_text() {
        let content = "这是一段普通文本";
        let blocks = SessionService::parse_content_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].r#type, "text");
        assert_eq!(blocks[0].content, content);
    }

    #[test]
    fn test_parse_content_blocks_with_code() {
        let content = "这是文本\n```rust\nfn main() {}\n```\n这是更多文本";
        let blocks = SessionService::parse_content_blocks(content);

        assert_eq!(blocks.len(), 3);

        assert_eq!(blocks[0].r#type, "text");
        assert!(blocks[0].content.contains("这是文本"));

        assert_eq!(blocks[1].r#type, "code");
        assert_eq!(blocks[1].language, Some("rust".to_string()));
        assert!(blocks[1].content.contains("fn main()"));

        assert_eq!(blocks[2].r#type, "text");
        assert!(blocks[2].content.contains("这是更多文本"));
    }

    #[test]
    fn test_parse_content_blocks_code_without_language() {
        let content = "```\nsome code\n```";
        let blocks = SessionService::parse_content_blocks(content);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].r#type, "code");
        assert_eq!(blocks[0].language, None);
    }
}
