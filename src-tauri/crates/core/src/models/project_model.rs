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
#[serde(rename_all = "camelCase")]
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
    /// 音频
    Audio,
    /// 视频
    Video,
    /// 纯文本
    Text,
    /// 数据文件（CSV、JSON 等）
    Data,
    /// 链接
    Link,
    /// 图标（海报扩展）
    Icon,
    /// 配色方案（海报扩展）
    Color,
    /// 布局模板（海报扩展）
    Layout,
}

impl Default for MaterialType {
    fn default() -> Self {
        Self::Document
    }
}

#[allow(dead_code)]
impl MaterialType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaterialType::Document => "document",
            MaterialType::Image => "image",
            MaterialType::Audio => "audio",
            MaterialType::Video => "video",
            MaterialType::Text => "text",
            MaterialType::Data => "data",
            MaterialType::Link => "link",
            MaterialType::Icon => "icon",
            MaterialType::Color => "color",
            MaterialType::Layout => "layout",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "document" => MaterialType::Document,
            "image" => MaterialType::Image,
            "audio" => MaterialType::Audio,
            "video" => MaterialType::Video,
            "text" => MaterialType::Text,
            "data" => MaterialType::Data,
            "link" => MaterialType::Link,
            "icon" => MaterialType::Icon,
            "color" => MaterialType::Color,
            "layout" => MaterialType::Layout,
            _ => MaterialType::Document,
        }
    }

    /// 判断是否为海报素材类型
    pub fn is_poster_material(&self) -> bool {
        matches!(
            self,
            MaterialType::Image | MaterialType::Icon | MaterialType::Color | MaterialType::Layout
        )
    }
}

/// 图片分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageCategory {
    /// 背景图
    Background,
    /// 产品图
    Product,
    /// 人物图
    Person,
    /// 装饰图
    Decoration,
    /// 纹理图
    Texture,
    /// 其他
    Other,
}

impl Default for ImageCategory {
    fn default() -> Self {
        Self::Other
    }
}

#[allow(dead_code)]
impl ImageCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageCategory::Background => "background",
            ImageCategory::Product => "product",
            ImageCategory::Person => "person",
            ImageCategory::Decoration => "decoration",
            ImageCategory::Texture => "texture",
            ImageCategory::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "background" => ImageCategory::Background,
            "product" => ImageCategory::Product,
            "person" => ImageCategory::Person,
            "decoration" => ImageCategory::Decoration,
            "texture" => ImageCategory::Texture,
            _ => ImageCategory::Other,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ImageCategory::Background => "背景",
            ImageCategory::Product => "产品",
            ImageCategory::Person => "人物",
            ImageCategory::Decoration => "装饰",
            ImageCategory::Texture => "纹理",
            ImageCategory::Other => "其他",
        }
    }
}

/// 布局分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutCategory {
    /// 大图型
    HeroImage,
    /// 文字主导
    TextDominant,
    /// 网格型
    Grid,
    /// 分割型
    Split,
    /// 极简型
    Minimal,
    /// 拼贴型
    Collage,
}

impl Default for LayoutCategory {
    fn default() -> Self {
        Self::HeroImage
    }
}

#[allow(dead_code)]
impl LayoutCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            LayoutCategory::HeroImage => "hero-image",
            LayoutCategory::TextDominant => "text-dominant",
            LayoutCategory::Grid => "grid",
            LayoutCategory::Split => "split",
            LayoutCategory::Minimal => "minimal",
            LayoutCategory::Collage => "collage",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hero-image" => LayoutCategory::HeroImage,
            "text-dominant" => LayoutCategory::TextDominant,
            "grid" => LayoutCategory::Grid,
            "split" => LayoutCategory::Split,
            "minimal" => LayoutCategory::Minimal,
            "collage" => LayoutCategory::Collage,
            _ => LayoutCategory::HeroImage,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LayoutCategory::HeroImage => "大图型",
            LayoutCategory::TextDominant => "文字型",
            LayoutCategory::Grid => "网格型",
            LayoutCategory::Split => "分割型",
            LayoutCategory::Minimal => "极简型",
            LayoutCategory::Collage => "拼贴型",
        }
    }
}

/// 海报素材元数据
///
/// 存储海报素材的扩展信息，与 materials 表关联。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PosterMaterialMetadata {
    /// 关联的素材 ID
    pub material_id: String,
    /// 图片分类（仅 image 类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_category: Option<String>,
    /// 图片宽度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    /// 图片高度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    /// 缩略图路径或 base64
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// 主色列表（JSON 数组）
    #[serde(default)]
    pub colors: Vec<String>,
    /// 图标风格（仅 icon 类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_style: Option<String>,
    /// 图标分类（仅 icon 类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_category: Option<String>,
    /// 配色方案数据（仅 color 类型，JSON）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_scheme_json: Option<String>,
    /// 配色氛围（仅 color 类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    /// 布局分类（仅 layout 类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_category: Option<String>,
    /// 布局元素数量（仅 layout 类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_count: Option<i32>,
    /// 布局预览图
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// Fabric.js JSON（仅 layout 类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fabric_json: Option<String>,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 创建海报素材元数据请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePosterMetadataRequest {
    /// 关联的素材 ID
    pub material_id: String,
    /// 图片分类
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_category: Option<String>,
    /// 图片宽度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    /// 图片高度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    /// 缩略图
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// 主色列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<String>>,
    /// 图标风格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_style: Option<String>,
    /// 图标分类
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_category: Option<String>,
    /// 配色方案 JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_scheme_json: Option<String>,
    /// 配色氛围
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    /// 布局分类
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_category: Option<String>,
    /// 布局元素数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_count: Option<i32>,
    /// 布局预览图
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// Fabric.js JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fabric_json: Option<String>,
}

/// 海报素材（完整视图）
///
/// 包含基础素材和海报扩展元数据的完整数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PosterMaterial {
    /// 基础素材
    #[serde(flatten)]
    pub base: Material,
    /// 海报元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PosterMaterialMetadata>,
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
#[serde(rename_all = "camelCase")]
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
// 品牌人设扩展类型
// ============================================================================

/// 品牌个性类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BrandPersonality {
    /// 专业严谨
    Professional,
    /// 亲切友好
    Friendly,
    /// 活泼有趣
    Playful,
    /// 奢华高端
    Luxurious,
    /// 简约克制
    Minimalist,
    /// 大胆张扬
    Bold,
    /// 优雅精致
    Elegant,
}

impl Default for BrandPersonality {
    fn default() -> Self {
        Self::Professional
    }
}

#[allow(dead_code)]
impl BrandPersonality {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrandPersonality::Professional => "professional",
            BrandPersonality::Friendly => "friendly",
            BrandPersonality::Playful => "playful",
            BrandPersonality::Luxurious => "luxurious",
            BrandPersonality::Minimalist => "minimalist",
            BrandPersonality::Bold => "bold",
            BrandPersonality::Elegant => "elegant",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "professional" => BrandPersonality::Professional,
            "friendly" => BrandPersonality::Friendly,
            "playful" => BrandPersonality::Playful,
            "luxurious" => BrandPersonality::Luxurious,
            "minimalist" => BrandPersonality::Minimalist,
            "bold" => BrandPersonality::Bold,
            "elegant" => BrandPersonality::Elegant,
            _ => BrandPersonality::Professional,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            BrandPersonality::Professional => "专业严谨",
            BrandPersonality::Friendly => "亲切友好",
            BrandPersonality::Playful => "活泼有趣",
            BrandPersonality::Luxurious => "奢华高端",
            BrandPersonality::Minimalist => "简约克制",
            BrandPersonality::Bold => "大胆张扬",
            BrandPersonality::Elegant => "优雅精致",
        }
    }
}

/// 设计风格类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DesignStyle {
    /// 极简
    Minimal,
    /// 现代
    Modern,
    /// 经典
    Classic,
    /// 活泼
    Playful,
    /// 商务
    Corporate,
    /// 艺术
    Artistic,
    /// 复古
    Retro,
}

impl Default for DesignStyle {
    fn default() -> Self {
        Self::Modern
    }
}

#[allow(dead_code)]
impl DesignStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            DesignStyle::Minimal => "minimal",
            DesignStyle::Modern => "modern",
            DesignStyle::Classic => "classic",
            DesignStyle::Playful => "playful",
            DesignStyle::Corporate => "corporate",
            DesignStyle::Artistic => "artistic",
            DesignStyle::Retro => "retro",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "minimal" => DesignStyle::Minimal,
            "modern" => DesignStyle::Modern,
            "classic" => DesignStyle::Classic,
            "playful" => DesignStyle::Playful,
            "corporate" => DesignStyle::Corporate,
            "artistic" => DesignStyle::Artistic,
            "retro" => DesignStyle::Retro,
            _ => DesignStyle::Modern,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DesignStyle::Minimal => "极简",
            DesignStyle::Modern => "现代",
            DesignStyle::Classic => "经典",
            DesignStyle::Playful => "活泼",
            DesignStyle::Corporate => "商务",
            DesignStyle::Artistic => "艺术",
            DesignStyle::Retro => "复古",
        }
    }
}

/// 配色方案
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorScheme {
    /// 主色
    pub primary: String,
    /// 辅色
    pub secondary: String,
    /// 强调色
    pub accent: String,
    /// 背景色
    pub background: String,
    /// 文字色
    pub text: String,
    /// 次要文字色
    pub text_secondary: String,
    /// 渐变配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradients: Option<Vec<GradientConfig>>,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            primary: "#2196F3".to_string(),
            secondary: "#90CAF9".to_string(),
            accent: "#1976D2".to_string(),
            background: "#FFFFFF".to_string(),
            text: "#212121".to_string(),
            text_secondary: "#757575".to_string(),
            gradients: None,
        }
    }
}

/// 渐变配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientConfig {
    /// 渐变名称
    pub name: String,
    /// 渐变颜色列表
    pub colors: Vec<String>,
    /// 渐变方向（角度）
    pub direction: i32,
}

/// 字体方案
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Typography {
    /// 标题字体
    pub title_font: String,
    /// 标题字重
    pub title_weight: i32,
    /// 正文字体
    pub body_font: String,
    /// 正文字重
    pub body_weight: i32,
    /// 标题字号基准
    pub title_size: i32,
    /// 正文字号基准
    pub body_size: i32,
    /// 行高
    pub line_height: f32,
    /// 字间距
    pub letter_spacing: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            title_font: "思源黑体".to_string(),
            title_weight: 700,
            body_font: "苹方".to_string(),
            body_weight: 400,
            title_size: 72,
            body_size: 24,
            line_height: 1.5,
            letter_spacing: 0.0,
        }
    }
}

/// Logo 位置配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoPlacement {
    /// 默认位置
    pub default_position: String,
    /// 内边距
    pub padding: i32,
    /// 最大尺寸（百分比）
    pub max_size: i32,
}

impl Default for LogoPlacement {
    fn default() -> Self {
        Self {
            default_position: "top-left".to_string(),
            padding: 20,
            max_size: 15,
        }
    }
}

/// 图片风格配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageStyle {
    /// CSS 滤镜
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// 圆角
    pub border_radius: i32,
    /// 阴影
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<String>,
    /// 偏好比例
    pub preferred_ratio: String,
}

impl Default for ImageStyle {
    fn default() -> Self {
        Self {
            filter: None,
            border_radius: 8,
            shadow: None,
            preferred_ratio: "3:4".to_string(),
        }
    }
}

/// 图标风格配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconStyle {
    /// 风格类型
    pub style: String,
    /// 描边宽度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<i32>,
    /// 默认颜色
    pub default_color: String,
}

impl Default for IconStyle {
    fn default() -> Self {
        Self {
            style: "outlined".to_string(),
            stroke_width: Some(2),
            default_color: "#333333".to_string(),
        }
    }
}

/// 品牌调性配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandTone {
    /// 品牌关键词
    #[serde(default)]
    pub keywords: Vec<String>,
    /// 品牌个性
    pub personality: String,
    /// 品牌语调
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_tone: Option<String>,
    /// 目标受众描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_audience: Option<String>,
}

impl Default for BrandTone {
    fn default() -> Self {
        Self {
            keywords: vec![],
            personality: "professional".to_string(),
            voice_tone: None,
            target_audience: None,
        }
    }
}

/// 设计配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignConfig {
    /// 主风格
    pub primary_style: String,
    /// 配色方案
    pub color_scheme: ColorScheme,
    /// 字体方案
    pub typography: Typography,
}

impl Default for DesignConfig {
    fn default() -> Self {
        Self {
            primary_style: "modern".to_string(),
            color_scheme: ColorScheme::default(),
            typography: Typography::default(),
        }
    }
}

/// 视觉规范配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct VisualConfig {
    /// Logo 图片 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    /// Logo 位置配置
    pub logo_placement: LogoPlacement,
    /// 图片风格
    pub image_style: ImageStyle,
    /// 图标风格
    pub icon_style: IconStyle,
    /// 装饰元素列表
    #[serde(default)]
    pub decorations: Vec<String>,
}

/// 品牌人设扩展
///
/// 存储品牌人设的海报设计专用字段，与基础 Persona 关联。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandPersonaExtension {
    /// 关联的人设 ID
    pub persona_id: String,
    /// 品牌调性
    pub brand_tone: BrandTone,
    /// 设计配置
    pub design: DesignConfig,
    /// 视觉规范
    pub visual: VisualConfig,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 创建品牌人设扩展请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBrandExtensionRequest {
    /// 关联的人设 ID
    pub persona_id: String,
    /// 品牌调性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand_tone: Option<BrandTone>,
    /// 设计配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design: Option<DesignConfig>,
    /// 视觉规范
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual: Option<VisualConfig>,
}

/// 更新品牌人设扩展请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBrandExtensionRequest {
    /// 品牌调性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand_tone: Option<BrandTone>,
    /// 设计配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design: Option<DesignConfig>,
    /// 视觉规范
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual: Option<VisualConfig>,
}

/// 品牌人设（完整视图）
///
/// 包含基础人设和品牌扩展的完整数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandPersona {
    /// 基础人设
    #[serde(flatten)]
    pub base: Persona,
    /// 品牌调性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand_tone: Option<BrandTone>,
    /// 设计配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design: Option<DesignConfig>,
    /// 视觉规范
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual: Option<VisualConfig>,
}

/// 品牌人设模板
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandPersonaTemplate {
    /// 模板 ID
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 品牌调性
    pub brand_tone: BrandTone,
    /// 设计配置
    pub design: DesignConfig,
    /// 视觉规范
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual: Option<VisualConfig>,
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
        assert_eq!(MaterialType::Audio.as_str(), "audio");
        assert_eq!(MaterialType::Video.as_str(), "video");
        assert_eq!(MaterialType::Text.as_str(), "text");
        assert_eq!(MaterialType::Data.as_str(), "data");
        assert_eq!(MaterialType::Link.as_str(), "link");
        assert_eq!(MaterialType::Icon.as_str(), "icon");
        assert_eq!(MaterialType::Color.as_str(), "color");
        assert_eq!(MaterialType::Layout.as_str(), "layout");

        assert_eq!(MaterialType::from_str("document"), MaterialType::Document);
        assert_eq!(MaterialType::from_str("IMAGE"), MaterialType::Image);
        assert_eq!(MaterialType::from_str("audio"), MaterialType::Audio);
        assert_eq!(MaterialType::from_str("VIDEO"), MaterialType::Video);
        assert_eq!(MaterialType::from_str("icon"), MaterialType::Icon);
        assert_eq!(MaterialType::from_str("color"), MaterialType::Color);
        assert_eq!(MaterialType::from_str("layout"), MaterialType::Layout);
        assert_eq!(MaterialType::from_str("unknown"), MaterialType::Document);
    }

    #[test]
    fn test_material_type_is_poster_material() {
        assert!(MaterialType::Image.is_poster_material());
        assert!(MaterialType::Icon.is_poster_material());
        assert!(MaterialType::Color.is_poster_material());
        assert!(MaterialType::Layout.is_poster_material());
        assert!(!MaterialType::Document.is_poster_material());
        assert!(!MaterialType::Audio.is_poster_material());
        assert!(!MaterialType::Video.is_poster_material());
        assert!(!MaterialType::Text.is_poster_material());
        assert!(!MaterialType::Data.is_poster_material());
        assert!(!MaterialType::Link.is_poster_material());
    }

    #[test]
    fn test_image_category_conversion() {
        assert_eq!(ImageCategory::Background.as_str(), "background");
        assert_eq!(ImageCategory::Product.as_str(), "product");
        assert_eq!(ImageCategory::Person.as_str(), "person");

        assert_eq!(
            ImageCategory::from_str("background"),
            ImageCategory::Background
        );
        assert_eq!(ImageCategory::from_str("PRODUCT"), ImageCategory::Product);
        assert_eq!(ImageCategory::from_str("unknown"), ImageCategory::Other);

        assert_eq!(ImageCategory::Background.display_name(), "背景");
        assert_eq!(ImageCategory::Product.display_name(), "产品");
    }

    #[test]
    fn test_layout_category_conversion() {
        assert_eq!(LayoutCategory::HeroImage.as_str(), "hero-image");
        assert_eq!(LayoutCategory::TextDominant.as_str(), "text-dominant");
        assert_eq!(LayoutCategory::Grid.as_str(), "grid");

        assert_eq!(
            LayoutCategory::from_str("hero-image"),
            LayoutCategory::HeroImage
        );
        assert_eq!(LayoutCategory::from_str("grid"), LayoutCategory::Grid);
        assert_eq!(
            LayoutCategory::from_str("unknown"),
            LayoutCategory::HeroImage
        );

        assert_eq!(LayoutCategory::HeroImage.display_name(), "大图型");
        assert_eq!(LayoutCategory::Grid.display_name(), "网格型");
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

    #[test]
    fn test_brand_personality_conversion() {
        assert_eq!(BrandPersonality::Professional.as_str(), "professional");
        assert_eq!(BrandPersonality::Friendly.as_str(), "friendly");
        assert_eq!(BrandPersonality::Playful.as_str(), "playful");

        assert_eq!(
            BrandPersonality::from_str("professional"),
            BrandPersonality::Professional
        );
        assert_eq!(
            BrandPersonality::from_str("FRIENDLY"),
            BrandPersonality::Friendly
        );
        assert_eq!(
            BrandPersonality::from_str("unknown"),
            BrandPersonality::Professional
        );
    }

    #[test]
    fn test_brand_personality_display_name() {
        assert_eq!(BrandPersonality::Professional.display_name(), "专业严谨");
        assert_eq!(BrandPersonality::Friendly.display_name(), "亲切友好");
        assert_eq!(BrandPersonality::Luxurious.display_name(), "奢华高端");
    }

    #[test]
    fn test_design_style_conversion() {
        assert_eq!(DesignStyle::Minimal.as_str(), "minimal");
        assert_eq!(DesignStyle::Modern.as_str(), "modern");
        assert_eq!(DesignStyle::Corporate.as_str(), "corporate");

        assert_eq!(DesignStyle::from_str("minimal"), DesignStyle::Minimal);
        assert_eq!(DesignStyle::from_str("MODERN"), DesignStyle::Modern);
        assert_eq!(DesignStyle::from_str("unknown"), DesignStyle::Modern);
    }

    #[test]
    fn test_color_scheme_serialization() {
        let scheme = ColorScheme::default();
        let json = serde_json::to_string(&scheme).unwrap();
        let parsed: ColorScheme = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.primary, "#2196F3");
        assert_eq!(parsed.background, "#FFFFFF");
    }

    #[test]
    fn test_typography_serialization() {
        let typography = Typography::default();
        let json = serde_json::to_string(&typography).unwrap();
        let parsed: Typography = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.title_font, "思源黑体");
        assert_eq!(parsed.title_weight, 700);
        assert_eq!(parsed.body_font, "苹方");
    }

    #[test]
    fn test_brand_persona_extension_serialization() {
        let extension = BrandPersonaExtension {
            persona_id: "persona-1".to_string(),
            brand_tone: BrandTone::default(),
            design: DesignConfig::default(),
            visual: VisualConfig::default(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let json = serde_json::to_string(&extension).unwrap();
        let parsed: BrandPersonaExtension = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.persona_id, "persona-1");
        assert_eq!(parsed.brand_tone.personality, "professional");
        assert_eq!(parsed.design.primary_style, "modern");
    }

    #[test]
    fn test_brand_defaults() {
        assert_eq!(BrandPersonality::default(), BrandPersonality::Professional);
        assert_eq!(DesignStyle::default(), DesignStyle::Modern);

        let color_scheme = ColorScheme::default();
        assert_eq!(color_scheme.primary, "#2196F3");

        let typography = Typography::default();
        assert_eq!(typography.title_size, 72);
        assert_eq!(typography.body_size, 24);
    }
}
