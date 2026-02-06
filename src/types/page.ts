/**
 * 页面类型定义
 *
 * 支持静态页面和动态插件页面
 * - 静态页面: 预定义的页面标识符
 * - 动态插件页面: `plugin:${string}` 格式，如 "plugin:machine-id-tool"
 *
 * @module types/page
 */

export type Page =
  | "provider-pool"
  | "api-server"
  | "agent"
  | "image-gen"
  | "mcp"
  | "tools"
  | "plugins"
  | "settings"
  | "terminal"
  | "sysinfo"
  | "files"
  | "web"
  | "image-analysis"
  | "projects"
  | "project-detail"
  | "vibe"
  | `plugin:${string}`;

/**
 * Agent 页面参数
 * 用于从项目入口跳转到创作界面时传递项目上下文
 */
export interface AgentPageParams {
  projectId?: string;
  contentId?: string;
}

/**
 * 项目详情页参数
 */
export interface ProjectDetailPageParams {
  projectId: string;
}

/**
 * 页面参数联合类型
 */
export type PageParams =
  | AgentPageParams
  | ProjectDetailPageParams
  | Record<string, unknown>;
