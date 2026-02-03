//! 项目相关模型定义
//!
//! 定义统一内容创作系统中的项目相关数据结构，包括：
//! - Persona（人设）
//! - Material（素材）
//! - Template（排版模板）
//! - PublishConfig（发布配置）
//! - ProjectContext（项目上下文）
//!
//! 以及相关的请求类型。

use serde::{Deserialize, Serialize};

// ============================================================================
// 人设相关类型
// ============================================================================

/// 人设配置
///
/// 存储项目级人设配置，包含写作风格、语气、目标读者等信息。
/// 用于 AI 生成内容时的风格指导。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    /// 唯一标识
    pub id: String,
    /// 所属项目 ID
    pub project_id: String,
    /// 人设名称
    pub name: String,
    /// 人设描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 写作风格（如：专业、轻松、幽默等）
    pub style: String,
    /// 语气（如：正式、亲切、活泼等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    /// 目标读者群体
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_audience: Option<String>,
    /// 禁用词列表
    #[serde(default)]
    pub forbidden_words: Vec<String>,
    /// 偏好词列表
    #[serde(default)]
    pub preferred_words: Vec<String>,
    /// 示例文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<String>,
    /// 适用平台列表
    #[serde(default)]
    pub platforms: Vec<String>,
    /// 是否为项目默认人设
    #[serde(default)]
    pub is_default: bool,
    /// 创建时间（Unix 时间戳）
    pub created_at: i64,
    /// 更新时间（Unix 时间戳）
    pub updated_at: i64,
}

/// 创建人设请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePersonaRequest {
    /// 所属项目 ID
    pub project_id: String,
    /// 人设名称
    pub name: String,
    /// 人设描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 写作风格
    pub style: String,
    /// 语气
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    /// 目标读者群体
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_audience: Option<String>,
    /// 禁用词列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forbidden_words: Option<Vec<String>>,
    /// 偏好词列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_words: Option<Vec<String>>,
    /// 示例文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<String>,
    /// 适用平台列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<String>>,
}

/// 更新人设请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonaUpdate {
    /// 人设名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 人设描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 写作风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// 语气
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    /// 目标读者群体
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_audience: Option<String>,
    /// 禁用词列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forbidden_words: Option<Vec<String>>,
    /// 偏好词列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_words: Option<Vec<String>>,
    /// 示例文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<String>,
    /// 适用平台列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<String>>,
}

/// 人设模板
///
/// 预定义的人设模板，用于快速创建人设。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaTemplate {
    /// 模板 ID
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 写作风格
    pub style: String,
    /// 语气
    pub tone: String,
    /// 目标读者群体
    pub target_audience: String,
    /// 适用平台列表
    #[serde(default)]
    pub platforms: Vec<String>,
}

// ============================================================================
// 素材相关类型
// ============================================================================

/// 素材类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MaterialType {
    /// 文档（PDF、Word 等）
    Document,
    /// 图片
    Image,
    /// 纯文本
    Text,
    /// 数据文件（CSV、JSON 等）
    Data,
    /// 链接
    Link,
}

impl Default for MaterialType {
    fn default() -> Self {
        Self::Document
    }
}

impl MaterialType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaterialType::Document => "document",
            MaterialType::Image => "image",
            MaterialType::Text => "text",
            MaterialType::Data => "data",
            MaterialType::Link => "link",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "document" => MaterialType::Document,
            "image" => MaterialType::Image,
            "text" => MaterialType::Text,
            "data" => MaterialType::Data,
            "link" => MaterialType::Link,
            _ => MaterialType::Document,
        }
    }
}

/// 素材
///
/// 存储项目级素材，包含文档、图片、文本等参考资料。
/// 用于 AI 创作时的引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    /// 唯一标识
    pub id: String,
    /// 所属项目 ID
    pub project_id: String,
    /// 素材名称
    pub name: String,
    /// 素材类型
    #[serde(rename = "type")]
    pub material_type: String,
    /// 文件路径（本地存储路径）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// 文件大小（字节）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    /// MIME 类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// 文本内容（用于 text 类型或提取的内容）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// 标签列表
    #[serde(default)]
    pub tags: Vec<String>,
    /// 素材描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 创建时间（Unix 时间戳）
    pub created_at: i64,
}

/// 上传素材请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMaterialRequest {
    /// 所属项目 ID
    pub project_id: String,
    /// 素材名称
    pub name: String,
    /// 素材类型
    #[serde(rename = "type")]
    pub material_type: String,
    /// 文件路径（上传的临时文件路径）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// 文本内容（用于 text 类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// 标签列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// 素材描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 更新素材请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialUpdate {
    /// 素材名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 标签列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// 素材描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 素材筛选条件
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialFilter {
    /// 按类型筛选
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub material_type: Option<String>,
    /// 按标签筛选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// 搜索关键词
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<String>,
}

// ============================================================================
// 排版模板相关类型
// ============================================================================

/// 平台类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// 小红书
    Xiaohongshu,
    /// 微信公众号
    Wechat,
    /// 知乎
    Zhihu,
    /// 微博
    Weibo,
    /// 抖音
    Douyin,
    /// Markdown 通用格式
    Markdown,
}

impl Default for Platform {
    fn default() -> Self {
        Self::Markdown
    }
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Xiaohongshu => "xiaohongshu",
            Platform::Wechat => "wechat",
            Platform::Zhihu => "zhihu",
            Platform::Weibo => "weibo",
            Platform::Douyin => "douyin",
            Platform::Markdown => "markdown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "xiaohongshu" => Platform::Xiaohongshu,
            "wechat" => Platform::Wechat,
            "zhihu" => Platform::Zhihu,
            "weibo" => Platform::Weibo,
            "douyin" => Platform::Douyin,
            "markdown" => Platform::Markdown,
            _ => Platform::Markdown,
        }
    }

    /// 获取平台显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Platform::Xiaohongshu => "小红书",
            Platform::Wechat => "微信公众号",
            Platform::Zhihu => "知乎",
            Platform::Weibo => "微博",
            Platform::Douyin => "抖音",
            Platform::Markdown => "Markdown",
        }
    }
}

/// Emoji 使用程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmojiUsage {
    /// 大量使用
    Heavy,
    /// 适度使用
    Moderate,
    /// 少量使用
    Minimal,
}

impl Default for EmojiUsage {
    fn default() -> Self {
        Self::Moderate
    }
}

impl EmojiUsage {
    pub fn as_str(&self) -> &'static str {
        match self {
            EmojiUsage::Heavy => "heavy",
            EmojiUsage::Moderate => "moderate",
            EmojiUsage::Minimal => "minimal",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "heavy" => EmojiUsage::Heavy,
            "moderate" => EmojiUsage::Moderate,
            "minimal" => EmojiUsage::Minimal,
            _ => EmojiUsage::Moderate,
        }
    }
}

/// 排版模板
///
/// 存储项目级排版模板，定义输出内容的格式规则。
/// 用于 AI 生成内容时的格式指导。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// 唯一标识
    pub id: String,
    /// 所属项目 ID
    pub project_id: String,
    /// 模板名称
    pub name: String,
    /// 目标平台
    pub platform: String,
    /// 标题风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_style: Option<String>,
    /// 段落风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph_style: Option<String>,
    /// 结尾风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_style: Option<String>,
    /// Emoji 使用程度
    pub emoji_usage: String,
    /// 话题标签规则
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtag_rules: Option<String>,
    /// 图片规则
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_rules: Option<String>,
    /// 是否为项目默认模板
    #[serde(default)]
    pub is_default: bool,
    /// 创建时间（Unix 时间戳）
    pub created_at: i64,
    /// 更新时间（Unix 时间戳）
    pub updated_at: i64,
}

/// 创建模板请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    /// 所属项目 ID
    pub project_id: String,
    /// 模板名称
    pub name: String,
    /// 目标平台
    pub platform: String,
    /// 标题风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_style: Option<String>,
    /// 段落风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph_style: Option<String>,
    /// 结尾风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_style: Option<String>,
    /// Emoji 使用程度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_usage: Option<String>,
    /// 话题标签规则
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtag_rules: Option<String>,
    /// 图片规则
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_rules: Option<String>,
}

/// 更新模板请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateUpdate {
    /// 模板名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 标题风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_style: Option<String>,
    /// 段落风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph_style: Option<String>,
    /// 结尾风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_style: Option<String>,
    /// Emoji 使用程度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_usage: Option<String>,
    /// 话题标签规则
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtag_rules: Option<String>,
    /// 图片规则
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_rules: Option<String>,
}

// ============================================================================
// 发布配置相关类型
// ============================================================================

/// 发布配置
///
/// 存储项目级发布配置，包含平台认证信息和发布历史。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishConfig {
    /// 唯一标识
    pub id: String,
    /// 所属项目 ID
    pub project_id: String,
    /// 目标平台
    pub platform: String,
    /// 是否已配置
    #[serde(default)]
    pub is_configured: bool,
    /// 最后发布时间（Unix 时间戳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_published_at: Option<i64>,
    /// 发布次数
    #[serde(default)]
    pub publish_count: i64,
    /// 创建时间（Unix 时间戳）
    pub created_at: i64,
    /// 更新时间（Unix 时间戳）
    pub updated_at: i64,
}

// ============================================================================
// 项目上下文相关类型
// ============================================================================

/// 项目上下文
///
/// 聚合项目的所有配置信息，用于注入到 AI System Prompt。
/// 包含项目基本信息、默认人设、素材列表和默认模板。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    /// 项目信息
    pub project: crate::workspace::Workspace,
    /// 默认人设（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<Persona>,
    /// 素材列表
    #[serde(default)]
    pub materials: Vec<Material>,
    /// 默认模板（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_type_conversion() {
        assert_eq!(MaterialType::Document.as_str(), "document");
        assert_eq!(MaterialType::Image.as_str(), "image");
        assert_eq!(MaterialType::Text.as_str(), "text");
        assert_eq!(MaterialType::Data.as_str(), "data");
        assert_eq!(MaterialType::Link.as_str(), "link");

        assert_eq!(MaterialType::from_str("document"), MaterialType::Document);
        assert_eq!(MaterialType::from_str("IMAGE"), MaterialType::Image);
        assert_eq!(MaterialType::from_str("unknown"), MaterialType::Document);
    }

    #[test]
    fn test_platform_conversion() {
        assert_eq!(Platform::Xiaohongshu.as_str(), "xiaohongshu");
        assert_eq!(Platform::Wechat.as_str(), "wechat");
        assert_eq!(Platform::Markdown.as_str(), "markdown");

        assert_eq!(Platform::from_str("xiaohongshu"), Platform::Xiaohongshu);
        assert_eq!(Platform::from_str("WECHAT"), Platform::Wechat);
        assert_eq!(Platform::from_str("unknown"), Platform::Markdown);
    }

    #[test]
    fn test_platform_display_name() {
        assert_eq!(Platform::Xiaohongshu.display_name(), "小红书");
        assert_eq!(Platform::Wechat.display_name(), "微信公众号");
        assert_eq!(Platform::Markdown.display_name(), "Markdown");
    }

    #[test]
    fn test_emoji_usage_conversion() {
        assert_eq!(EmojiUsage::Heavy.as_str(), "heavy");
        assert_eq!(EmojiUsage::Moderate.as_str(), "moderate");
        assert_eq!(EmojiUsage::Minimal.as_str(), "minimal");

        assert_eq!(EmojiUsage::from_str("heavy"), EmojiUsage::Heavy);
        assert_eq!(EmojiUsage::from_str("MODERATE"), EmojiUsage::Moderate);
        assert_eq!(EmojiUsage::from_str("unknown"), EmojiUsage::Moderate);
    }

    #[test]
    fn test_persona_serialization() {
        let persona = Persona {
            id: "test-id".to_string(),
            project_id: "project-1".to_string(),
            name: "测试人设".to_string(),
            description: Some("这是一个测试人设".to_string()),
            style: "专业".to_string(),
            tone: Some("正式".to_string()),
            target_audience: Some("技术人员".to_string()),
            forbidden_words: vec!["禁词1".to_string()],
            preferred_words: vec!["偏好词1".to_string()],
            examples: None,
            platforms: vec!["xiaohongshu".to_string()],
            is_default: false,
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let json = serde_json::to_string(&persona).unwrap();
        let parsed: Persona = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, persona.id);
        assert_eq!(parsed.name, persona.name);
        assert_eq!(parsed.style, persona.style);
    }

    #[test]
    fn test_material_serialization() {
        let material = Material {
            id: "mat-1".to_string(),
            project_id: "project-1".to_string(),
            name: "测试素材.pdf".to_string(),
            material_type: "document".to_string(),
            file_path: Some("/path/to/file.pdf".to_string()),
            file_size: Some(1024),
            mime_type: Some("application/pdf".to_string()),
            content: None,
            tags: vec!["标签1".to_string()],
            description: Some("测试描述".to_string()),
            created_at: 1234567890,
        };

        let json = serde_json::to_string(&material).unwrap();
        assert!(json.contains("\"type\":\"document\""));

        let parsed: Material = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.material_type, "document");
    }

    #[test]
    fn test_template_serialization() {
        let template = Template {
            id: "tpl-1".to_string(),
            project_id: "project-1".to_string(),
            name: "小红书模板".to_string(),
            platform: "xiaohongshu".to_string(),
            title_style: Some("吸引眼球".to_string()),
            paragraph_style: Some("简短有力".to_string()),
            ending_style: Some("引导互动".to_string()),
            emoji_usage: "heavy".to_string(),
            hashtag_rules: Some("3-5个相关话题".to_string()),
            image_rules: Some("配图要精美".to_string()),
            is_default: true,
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let json = serde_json::to_string(&template).unwrap();
        let parsed: Template = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.platform, "xiaohongshu");
        assert_eq!(parsed.emoji_usage, "heavy");
        assert!(parsed.is_default);
    }

    #[test]
    fn test_create_persona_request() {
        let req = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "新人设".to_string(),
            description: None,
            style: "轻松".to_string(),
            tone: Some("活泼".to_string()),
            target_audience: None,
            forbidden_words: Some(vec!["禁词".to_string()]),
            preferred_words: None,
            examples: None,
            platforms: Some(vec!["wechat".to_string()]),
        };

        let json = serde_json::to_string(&req).unwrap();
        let parsed: CreatePersonaRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.project_id, "project-1");
        assert_eq!(parsed.style, "轻松");
    }

    #[test]
    fn test_upload_material_request() {
        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "文档.pdf".to_string(),
            material_type: "document".to_string(),
            file_path: Some("/tmp/upload.pdf".to_string()),
            content: None,
            tags: Some(vec!["参考".to_string()]),
            description: Some("参考文档".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"type\":\"document\""));
    }

    #[test]
    fn test_default_values() {
        assert_eq!(MaterialType::default(), MaterialType::Document);
        assert_eq!(Platform::default(), Platform::Markdown);
        assert_eq!(EmojiUsage::default(), EmojiUsage::Moderate);
    }
}
