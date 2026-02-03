/**
 * 排版模板相关类型定义
 *
 * 定义排版模板（Template）相关的 TypeScript 类型。
 *
 * @module types/template
 * @requirements 8.3
 */

// ============================================================================
// 模板类型
// ============================================================================

/**
 * 平台类型枚举
 */
export type Platform =
  | "xiaohongshu" // 小红书
  | "wechat" // 微信公众号
  | "zhihu" // 知乎
  | "weibo" // 微博
  | "douyin" // 抖音
  | "markdown"; // Markdown

/**
 * 平台显示名称映射
 */
export const PlatformLabels: Record<Platform, string> = {
  xiaohongshu: "小红书",
  wechat: "微信公众号",
  zhihu: "知乎",
  weibo: "微博",
  douyin: "抖音",
  markdown: "Markdown",
};

/**
 * Emoji 使用程度枚举
 */
export type EmojiUsage = "heavy" | "moderate" | "minimal";

/**
 * Emoji 使用程度显示名称映射
 */
export const EmojiUsageLabels: Record<EmojiUsage, string> = {
  heavy: "大量使用",
  moderate: "适度使用",
  minimal: "少量使用",
};

/**
 * 排版模板
 */
export interface Template {
  id: string;
  projectId: string;
  name: string;
  platform: Platform;
  titleStyle?: string;
  paragraphStyle?: string;
  endingStyle?: string;
  emojiUsage: EmojiUsage;
  hashtagRules?: string;
  imageRules?: string;
  isDefault: boolean;
  createdAt: number;
  updatedAt: number;
}

/**
 * 创建模板请求
 */
export interface CreateTemplateRequest {
  projectId: string;
  name: string;
  platform: Platform;
  titleStyle?: string;
  paragraphStyle?: string;
  endingStyle?: string;
  emojiUsage?: EmojiUsage;
  hashtagRules?: string;
  imageRules?: string;
}

/**
 * 更新模板请求
 */
export interface TemplateUpdate {
  name?: string;
  titleStyle?: string;
  paragraphStyle?: string;
  endingStyle?: string;
  emojiUsage?: EmojiUsage;
  hashtagRules?: string;
  imageRules?: string;
}
