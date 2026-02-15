//! 统一记忆数据模型
//!
//! 定义了所有记忆条目的统一数据结构，支持多种记忆来源和分类

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 统一记忆条目
///
/// 这是所有记忆类型的基础结构，无论是从对话历史自动提取的还是手动创建的项目记忆
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMemory {
    /// 唯一标识符
    pub id: String,
    /// 所属会话 ID
    pub session_id: String,
    /// 记忆类型（对话/项目）
    pub memory_type: MemoryType,
    /// 记忆分类
    pub category: MemoryCategory,
    /// 记忆标题
    pub title: String,
    /// 记忆内容（详细）
    pub content: String,
    /// 记忆摘要（简短描述）
    pub summary: String,
    /// 标签列表
    pub tags: Vec<String>,
    /// 元数据
    pub metadata: MemoryMetadata,
    /// 创建时间（毫秒时间戳）
    pub created_at: i64,
    /// 更新时间（毫秒时间戳）
    pub updated_at: i64,
    /// 是否已归档
    pub archived: bool,
}

/// 记忆类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// 从对话历史自动提取
    Conversation,
    /// 项目相关的角色、世界观等
    Project,
}

/// 记忆分类（参考 LobeHub 的 5 层架构）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryCategory {
    /// 身份信息：关于你是谁的稳定信息
    Identity,
    /// 情境信息：对话背景与当前约束
    Context,
    /// 偏好信息：你的习惯、口味与偏爱
    Preference,
    /// 经验信息：过往经历与可复用知识
    Experience,
    /// 活动信息：近期计划与进行中的事项
    Activity,
}

/// 记忆元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// 置信度 (0.0 - 1.0)
    ///
    /// 表示记忆的可靠程度，自动提取的记忆通常较低（0.3-0.5），
    /// 手动创建或用户确认的记忆较高（0.7-1.0）
    pub confidence: f32,

    /// 重要性 (0-10)
    ///
    /// 0-2: 不重要，可以忽略
    /// 3-5: 一般重要，偶尔有用
    /// 6-8: 较重要，经常使用
    /// 9-10: 非常重要，必须记住
    pub importance: u8,

    /// 访问次数
    pub access_count: u32,

    /// 上次访问时间（毫秒时间戳）
    pub last_accessed_at: Option<i64>,

    /// 来源
    pub source: MemorySource,

    /// 向量嵌入（可选，用于语义搜索）
    ///
    /// 768 维向量（OpenAI text-embedding-3-small）
    /// 当 embedding 为 None 时，仅使用关键词搜索
    pub embedding: Option<Vec<f32>>,
}

/// 记忆来源
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    /// 自动从对话历史提取
    AutoExtracted,
    /// 手动创建
    Manual,
    /// 从外部导入
    Imported,
}

impl UnifiedMemory {
    /// 创建新的对话记忆
    pub fn new_conversation(
        session_id: String,
        category: MemoryCategory,
        title: String,
        content: String,
        summary: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            memory_type: MemoryType::Conversation,
            category,
            title,
            content,
            summary,
            tags: Vec::new(),
            metadata: MemoryMetadata {
                confidence: 0.5,
                importance: 5,
                access_count: 0,
                last_accessed_at: None,
                source: MemorySource::AutoExtracted,
                embedding: None,
            },
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            archived: false,
        }
    }

    /// 创建新的项目记忆
    pub fn new_project(
        session_id: String,
        category: MemoryCategory,
        title: String,
        content: String,
        summary: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            memory_type: MemoryType::Project,
            category,
            title,
            content,
            summary,
            tags: Vec::new(),
            metadata: MemoryMetadata {
                confidence: 0.8,
                importance: 6,
                access_count: 0,
                last_accessed_at: None,
                source: MemorySource::Manual,
                embedding: None,
            },
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            archived: false,
        }
    }

    /// 记录访问
    pub fn record_access(&mut self) {
        self.metadata.access_count += 1;
        self.metadata.last_accessed_at = Some(chrono::Utc::now().timestamp_millis());
    }

    /// 更新置信度
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.metadata.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// 更新重要性
    pub fn with_importance(mut self, importance: u8) -> Self {
        self.metadata.importance = importance.clamp(0, 10);
        self
    }

    /// 添加标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_conversation_memory() {
        let memory = UnifiedMemory::new_conversation(
            "session-123".to_string(),
            MemoryCategory::Preference,
            "喜欢咖啡".to_string(),
            "我喜欢喝黑咖啡，不加糖".to_string(),
            "偏好黑咖啡".to_string(),
        );

        assert_eq!(memory.memory_type, MemoryType::Conversation);
        assert_eq!(memory.category, MemoryCategory::Preference);
        assert_eq!(memory.metadata.confidence, 0.5);
        assert_eq!(memory.metadata.importance, 5);
        assert!(!memory.id.is_empty());
    }

    #[test]
    fn test_create_project_memory() {
        let memory = UnifiedMemory::new_project(
            "project-456".to_string(),
            MemoryCategory::Identity,
            "主角名字".to_string(),
            "主角叫张三".to_string(),
            "张三是主角".to_string(),
        );

        assert_eq!(memory.memory_type, MemoryType::Project);
        assert_eq!(memory.category, MemoryCategory::Identity);
        assert_eq!(memory.metadata.confidence, 0.8);
        assert_eq!(memory.metadata.importance, 6);
        assert!(!memory.id.is_empty());
    }

    #[test]
    fn test_record_access() {
        let mut memory = UnifiedMemory::new_conversation(
            "session-1".to_string(),
            MemoryCategory::Context,
            "测试".to_string(),
            "内容".to_string(),
            "摘要".to_string(),
        );

        assert_eq!(memory.metadata.access_count, 0);

        memory.record_access();
        assert_eq!(memory.metadata.access_count, 1);
        assert!(memory.metadata.last_accessed_at.is_some());
    }

    #[test]
    fn test_with_importance() {
        let memory = UnifiedMemory::new_conversation(
            "session-1".to_string(),
            MemoryCategory::Experience,
            "测试".to_string(),
            "内容".to_string(),
            "摘要".to_string(),
        )
        .with_importance(9);

        assert_eq!(memory.metadata.importance, 9);
    }

    #[test]
    fn test_confidence_clamping() {
        let memory1 = UnifiedMemory::new_conversation(
            "session-1".to_string(),
            MemoryCategory::Activity,
            "测试".to_string(),
            "内容".to_string(),
            "摘要".to_string(),
        )
        .with_confidence(1.5);

        assert_eq!(memory1.metadata.confidence, 1.0);

        let memory2 = UnifiedMemory::new_conversation(
            "session-1".to_string(),
            MemoryCategory::Activity,
            "测试".to_string(),
            "内容".to_string(),
            "摘要".to_string(),
        )
        .with_confidence(-0.5);

        assert_eq!(memory2.metadata.confidence, 0.0);
    }
}
