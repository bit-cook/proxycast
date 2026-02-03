/**
 * Workspace 相关类型定义
 *
 * @module types/workspace
 */

// ============================================================================
// Workspace 类型
// ============================================================================

/**
 * Workspace 类型枚举
 */
export type WorkspaceType =
  | "persistent" // 持久化项目
  | "social-media" // 社交媒体
  | "blog" // 博客
  | "novel" // 小说
  | "general"; // 通用

/**
 * Workspace 类型显示名称映射
 */
export const WorkspaceTypeLabels: Record<WorkspaceType, string> = {
  persistent: "持久化",
  "social-media": "社交媒体",
  blog: "博客",
  novel: "小说",
  general: "通用",
};
