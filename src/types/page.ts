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
  | "tools"
  | "plugins"
  | "settings"
  | "terminal"
  | "sysinfo"
  | "files"
  | "web"
  | `plugin:${string}`;
