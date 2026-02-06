/**
 * 外部工具 API
 *
 * 管理 Codex CLI 等外部命令行工具
 * 这些工具有自己的认证系统，不通过 ProxyCast 凭证池管理
 */

import { safeInvoke } from "@/lib/dev-bridge";

/**
 * Codex CLI 状态
 */
export interface CodexCliStatus {
  /** CLI 是否已安装 */
  installed: boolean;
  /** CLI 版本 */
  version?: string;
  /** 是否已登录 */
  logged_in: boolean;
  /** 登录方式（api_key 或 oauth） */
  auth_type?: "api_key" | "oauth" | "unknown";
  /** API Key 前缀（如果使用 API Key 登录） */
  api_key_prefix?: string;
  /** 错误信息 */
  error?: string;
}

/**
 * 外部工具信息
 */
export interface ExternalTool {
  /** 工具 ID */
  id: string;
  /** 显示名称 */
  name: string;
  /** 描述 */
  description: string;
  /** 是否已安装 */
  installed: boolean;
  /** 是否已配置/登录 */
  configured: boolean;
  /** 安装命令 */
  install_command: string;
  /** 配置命令 */
  config_command: string;
  /** 文档链接 */
  doc_url: string;
}

/**
 * 检查 Codex CLI 状态
 */
export async function checkCodexCliStatus(): Promise<CodexCliStatus> {
  return await safeInvoke("check_codex_cli_status");
}

/**
 * 获取 Codex CLI 登录命令
 */
export async function getCodexLoginCommand(): Promise<string> {
  return await safeInvoke("open_codex_cli_login");
}

/**
 * 获取 Codex CLI 登出命令
 */
export async function getCodexLogoutCommand(): Promise<string> {
  return await safeInvoke("open_codex_cli_logout");
}

/**
 * 获取所有外部工具列表
 */
export async function getExternalTools(): Promise<ExternalTool[]> {
  return await safeInvoke("get_external_tools");
}
