/**
 * Webview 管理 API
 *
 * 提供与 Tauri 后端 webview 命令交互的 TypeScript 接口。
 * 使用 Tauri 2.x 的 multiwebview 功能创建独立的浏览器窗口。
 *
 * @module lib/webview-api
 */

import { safeInvoke } from "@/lib/dev-bridge";
import { Webview } from "@tauri-apps/api/webview";

/**
 * Webview 面板信息
 */
export interface WebviewPanelInfo {
  /** 面板 ID */
  id: string;
  /** 当前 URL */
  url: string;
  /** 面板标题 */
  title: string;
  /** X 坐标 */
  x: number;
  /** Y 坐标 */
  y: number;
  /** 宽度 */
  width: number;
  /** 高度 */
  height: number;
}

/**
 * 创建 webview 面板的请求参数
 */
export interface CreateWebviewRequest {
  /** 面板 ID（唯一标识） */
  panel_id: string;
  /** 要加载的 URL */
  url: string;
  /** 面板标题（可选） */
  title?: string;
  /** X 坐标（相对于主窗口） */
  x: number;
  /** Y 坐标（相对于主窗口） */
  y: number;
  /** 宽度 */
  width: number;
  /** 高度 */
  height: number;
  /** Profile 隔离键（可选） */
  profile_key?: string;
  /** 是否启用持久化 profile（可选） */
  persistent_profile?: boolean;
}

/**
 * 创建 webview 面板的响应
 */
export interface CreateWebviewResponse {
  /** 是否成功 */
  success: boolean;
  /** 面板 ID */
  panel_id: string;
  /** 错误信息（如果有） */
  error?: string;
}

/**
 * 启动外部 Chrome Profile 的请求参数
 */
export interface OpenChromeProfileRequest {
  /** Profile 隔离键 */
  profile_key: string;
  /** 要打开的 URL */
  url: string;
}

/**
 * 启动外部 Chrome Profile 的响应
 */
export interface OpenChromeProfileResponse {
  /** 是否成功 */
  success: boolean;
  /** 是否复用已存在会话 */
  reused?: boolean;
  /** 浏览器来源 */
  browser_source?: "system" | "playwright";
  /** 浏览器可执行文件路径 */
  browser_path?: string;
  /** Profile 目录 */
  profile_dir?: string;
  /** 远程调试端口 */
  remote_debugging_port?: number;
  /** Chrome 进程 PID */
  pid?: number;
  /** DevTools HTTP URL */
  devtools_http_url?: string;
  /** 错误信息 */
  error?: string;
}

/**
 * Chrome Profile 会话信息
 */
export interface ChromeProfileSessionInfo {
  profile_key: string;
  browser_source: "system" | "playwright";
  browser_path: string;
  profile_dir: string;
  remote_debugging_port: number;
  pid: number;
  started_at: string;
  last_url: string;
}

export interface ChromeBridgeEndpointInfo {
  server_running: boolean;
  host: string;
  port: number;
  observer_ws_url: string;
  control_ws_url: string;
  bridge_key: string;
}

export interface ChromeBridgePageInfo {
  title?: string;
  url?: string;
  markdown: string;
  updated_at: string;
}

export interface ChromeBridgeObserverSnapshot {
  client_id: string;
  profile_key: string;
  connected_at: string;
  user_agent?: string;
  last_heartbeat_at?: string;
  last_page_info?: ChromeBridgePageInfo;
}

export interface ChromeBridgeControlSnapshot {
  client_id: string;
  connected_at: string;
  user_agent?: string;
}

export interface ChromeBridgePendingCommandSnapshot {
  request_id: string;
  source_type: "api" | "control";
  command: string;
  observer_client_id: string;
  wait_for_page_info: boolean;
  command_completed: boolean;
  created_at: string;
}

export interface ChromeBridgeStatusSnapshot {
  observer_count: number;
  control_count: number;
  pending_command_count: number;
  observers: ChromeBridgeObserverSnapshot[];
  controls: ChromeBridgeControlSnapshot[];
  pending_commands: ChromeBridgePendingCommandSnapshot[];
}

export interface ChromeBridgeCommandRequest {
  profile_key?: string;
  command: string;
  target?: string;
  text?: string;
  url?: string;
  wait_for_page_info?: boolean;
  timeout_ms?: number;
}

export interface ChromeBridgeCommandResult {
  success: boolean;
  request_id: string;
  command: string;
  message?: string;
  error?: string;
  page_info?: ChromeBridgePageInfo;
}

export type BrowserBackendType =
  | "aster_compat"
  | "proxycast_extension_bridge"
  | "cdp_direct";

export interface BrowserBackendPolicy {
  priority: BrowserBackendType[];
  auto_fallback: boolean;
}

export interface BrowserBackendStatusItem {
  backend: BrowserBackendType;
  available: boolean;
  reason?: string;
  capabilities: string[];
}

export interface BrowserBackendsStatusSnapshot {
  policy: BrowserBackendPolicy;
  bridge_observer_count: number;
  bridge_control_count: number;
  running_profile_count: number;
  cdp_alive_profile_count: number;
  aster_native_host_supported: boolean;
  aster_native_host_configured: boolean;
  backends: BrowserBackendStatusItem[];
}

export interface BrowserActionRequest {
  profile_key?: string;
  backend?: BrowserBackendType;
  action: string;
  args?: Record<string, unknown>;
  timeout_ms?: number;
}

export interface BrowserActionAttempt {
  backend: BrowserBackendType;
  success: boolean;
  message: string;
}

export interface BrowserActionResult {
  success: boolean;
  backend?: BrowserBackendType;
  action: string;
  request_id: string;
  data?: unknown;
  error?: string;
  attempts: BrowserActionAttempt[];
}

export interface BrowserActionAuditRecord {
  id: string;
  created_at: string;
  action: string;
  profile_key?: string;
  requested_backend?: BrowserBackendType;
  selected_backend?: BrowserBackendType;
  success: boolean;
  error?: string;
  attempts: BrowserActionAttempt[];
}

/**
 * 创建一个新的 webview 窗口来显示外部 URL
 *
 * @param request - 创建请求参数
 * @returns 创建结果
 */
export async function createWebviewPanel(
  request: CreateWebviewRequest,
): Promise<CreateWebviewResponse> {
  return safeInvoke<CreateWebviewResponse>("create_webview_panel", { request });
}

/**
 * 使用外部 Chrome + 独立 Profile 打开 URL
 */
export async function openChromeProfileWindow(
  request: OpenChromeProfileRequest,
): Promise<OpenChromeProfileResponse> {
  return safeInvoke<OpenChromeProfileResponse>("open_chrome_profile_window", {
    request,
  });
}

/**
 * 获取当前运行中的 Chrome Profile 会话
 */
export async function getChromeProfileSessions(): Promise<ChromeProfileSessionInfo[]> {
  return safeInvoke<ChromeProfileSessionInfo[]>("get_chrome_profile_sessions");
}

/**
 * 关闭指定的 Chrome Profile 会话
 */
export async function closeChromeProfileSession(
  profileKey: string,
): Promise<boolean> {
  return safeInvoke<boolean>("close_chrome_profile_session", {
    profile_key: profileKey,
  });
}

/**
 * 获取 ChromeBridge 端点信息（用于扩展配置）
 */
export async function getChromeBridgeEndpointInfo(): Promise<ChromeBridgeEndpointInfo> {
  return safeInvoke<ChromeBridgeEndpointInfo>("get_chrome_bridge_endpoint_info");
}

/**
 * 获取 ChromeBridge 当前连接状态
 */
export async function getChromeBridgeStatus(): Promise<ChromeBridgeStatusSnapshot> {
  return safeInvoke<ChromeBridgeStatusSnapshot>("get_chrome_bridge_status");
}

/**
 * 通过 ChromeBridge 发送测试命令
 */
export async function chromeBridgeExecuteCommand(
  request: ChromeBridgeCommandRequest,
): Promise<ChromeBridgeCommandResult> {
  return safeInvoke<ChromeBridgeCommandResult>("chrome_bridge_execute_command", {
    request,
  });
}

export async function getBrowserBackendPolicy(): Promise<BrowserBackendPolicy> {
  return safeInvoke<BrowserBackendPolicy>("get_browser_backend_policy");
}

export async function setBrowserBackendPolicy(
  policy: BrowserBackendPolicy,
): Promise<BrowserBackendPolicy> {
  return safeInvoke<BrowserBackendPolicy>("set_browser_backend_policy", {
    policy,
  });
}

export async function getBrowserBackendsStatus(): Promise<BrowserBackendsStatusSnapshot> {
  return safeInvoke<BrowserBackendsStatusSnapshot>("get_browser_backends_status");
}

export async function browserExecuteAction(
  request: BrowserActionRequest,
): Promise<BrowserActionResult> {
  return safeInvoke<BrowserActionResult>("browser_execute_action", { request });
}

export async function getBrowserActionAuditLogs(
  limit?: number,
): Promise<BrowserActionAuditRecord[]> {
  return safeInvoke<BrowserActionAuditRecord[]>("get_browser_action_audit_logs", {
    limit,
  });
}

/**
 * 关闭 webview 面板
 *
 * 尝试多种方法关闭 webview：
 * 1. 使用 Tauri JavaScript API 直接关闭
 * 2. 使用后端命令关闭
 *
 * @param panelId - 面板 ID
 * @returns 是否成功
 */
export async function closeWebviewPanel(panelId: string): Promise<boolean> {
  console.log("[webview-api] 尝试关闭 webview:", panelId);

  // 方法 1: 尝试使用 Tauri JavaScript API 直接关闭
  try {
    const webview = await Webview.getByLabel(panelId);
    if (webview) {
      console.log("[webview-api] 找到 webview，尝试关闭");
      await webview.close();
      console.log("[webview-api] Tauri API 关闭成功");
      // 也调用后端清理状态
      await safeInvoke<boolean>("close_webview_panel", {
        panel_id: panelId,
      }).catch(() => {});
      return true;
    }
  } catch (e) {
    console.warn("[webview-api] Tauri API 关闭失败:", e);
  }

  // 方法 2: 使用后端命令关闭
  try {
    const result = await safeInvoke<boolean>("close_webview_panel", {
      panel_id: panelId,
    });
    console.log("[webview-api] 后端命令关闭结果:", result);
    return result;
  } catch (e) {
    console.error("[webview-api] 后端命令关闭失败:", e);
    return false;
  }
}

/**
 * 导航到新 URL
 *
 * @param panelId - 面板 ID
 * @param url - 新 URL
 * @returns 是否成功
 */
export async function navigateWebviewPanel(
  panelId: string,
  url: string,
): Promise<boolean> {
  return safeInvoke<boolean>("navigate_webview_panel", {
    panel_id: panelId,
    url,
  });
}

/**
 * 获取所有活跃的 webview 面板
 *
 * @returns 面板列表
 */
export async function getWebviewPanels(): Promise<WebviewPanelInfo[]> {
  return safeInvoke<WebviewPanelInfo[]>("get_webview_panels");
}

/**
 * 聚焦指定的 webview 面板
 *
 * @param panelId - 面板 ID
 * @returns 是否成功
 */
export async function focusWebviewPanel(panelId: string): Promise<boolean> {
  return safeInvoke<boolean>("focus_webview_panel", { panel_id: panelId });
}

/**
 * 调整 webview 面板大小和位置
 *
 * @param panelId - 面板 ID
 * @param x - 新的 X 坐标
 * @param y - 新的 Y 坐标
 * @param width - 新的宽度
 * @param height - 新的高度
 * @returns 是否成功
 */
export async function resizeWebviewPanel(
  panelId: string,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<boolean> {
  return safeInvoke<boolean>("resize_webview_panel", {
    panel_id: panelId,
    x,
    y,
    width,
    height,
  });
}

/**
 * 生成唯一的面板 ID
 *
 * @returns 唯一 ID
 */
export function generatePanelId(): string {
  return `webview-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}
