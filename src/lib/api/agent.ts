/**
 * Agent API
 *
 * 原生 Rust Agent 的前端 API 封装
 * 支持流式输出和工具调用
 */

import { safeInvoke } from "@/lib/dev-bridge";

// ============================================================
// 流式事件类型 (Requirements: 9.1, 9.2, 9.3)
// ============================================================

/**
 * Token 使用量统计
 * Requirements: 9.5 - THE Frontend SHALL display token usage statistics after each Agent response
 */
export interface TokenUsage {
  /** 输入 token 数 */
  input_tokens: number;
  /** 输出 token 数 */
  output_tokens: number;
}

/**
 * 工具执行结果
 * Requirements: 9.2 - THE Frontend SHALL display a collapsible section showing the tool result
 */
export interface ToolExecutionResult {
  /** 是否成功 */
  success: boolean;
  /** 输出内容 */
  output: string;
  /** 错误信息（如果失败） */
  error?: string;
}

/**
 * 流式事件类型
 * Requirements: 9.1, 9.2, 9.3
 */
export type StreamEvent =
  | StreamEventTextDelta
  | StreamEventReasoningDelta
  | StreamEventToolStart
  | StreamEventToolEnd
  | StreamEventActionRequired
  | StreamEventDone
  | StreamEventFinalDone
  | StreamEventError;

/**
 * 文本增量事件
 * Requirements: 9.3 - THE Frontend SHALL distinguish between text responses and tool call responses visually
 */
export interface StreamEventTextDelta {
  type: "text_delta";
  text: string;
}

/**
 * 推理内容增量事件（DeepSeek reasoner 等模型的思考过程）
 * Requirements: 9.3 - THE Frontend SHALL distinguish between text responses and tool call responses visually
 */
export interface StreamEventReasoningDelta {
  type: "thinking_delta";
  text: string;
}

/**
 * 工具调用开始事件
 * Requirements: 9.1 - WHEN a tool is being executed, THE Frontend SHALL display a tool execution indicator with the tool name
 */
export interface StreamEventToolStart {
  type: "tool_start";
  /** 工具名称 */
  tool_name: string;
  /** 工具调用 ID */
  tool_id: string;
  /** 工具参数（JSON 字符串） */
  arguments?: string;
}

/**
 * 工具调用结束事件
 * Requirements: 9.2 - WHEN a tool completes, THE Frontend SHALL display a collapsible section showing the tool result
 */
export interface StreamEventToolEnd {
  type: "tool_end";
  /** 工具调用 ID */
  tool_id: string;
  /** 工具执行结果 */
  result: ToolExecutionResult;
}

/**
 * 权限确认请求事件
 * 当 Agent 需要用户确认某个操作时发送
 */
export interface StreamEventActionRequired {
  type: "action_required";
  /** 请求 ID */
  request_id: string;
  /** 操作类型 */
  action_type: "tool_confirmation" | "ask_user" | "elicitation";
  /** 工具名称（工具确认时） */
  tool_name?: string;
  /** 工具参数（工具确认时） */
  arguments?: Record<string, unknown>;
  /** 提示信息 */
  prompt?: string;
  /** 问题列表（ask_user 时） */
  questions?: Array<{
    question: string;
    header?: string;
    options?: Array<{
      label: string;
      description?: string;
    }>;
    multiSelect?: boolean;
  }>;
  /** 请求的数据结构（elicitation 时） */
  requested_schema?: Record<string, unknown>;
}

/**
 * 完成事件（单次 API 响应完成，工具循环可能继续）
 * Requirements: 9.5 - THE Frontend SHALL display token usage statistics after each Agent response
 */
export interface StreamEventDone {
  type: "done";
  /** Token 使用量（可选） */
  usage?: TokenUsage;
}

/**
 * 最终完成事件（整个对话完成，包括所有工具调用循环）
 * 前端收到此事件后才能取消监听
 */
export interface StreamEventFinalDone {
  type: "final_done";
  /** Token 使用量（可选） */
  usage?: TokenUsage;
}

/**
 * 错误事件
 */
export interface StreamEventError {
  type: "error";
  /** 错误信息 */
  message: string;
}

/**
 * 工具调用状态（用于 UI 显示）
 */
export interface ToolCallState {
  /** 工具调用 ID */
  id: string;
  /** 工具名称 */
  name: string;
  /** 工具参数（JSON 字符串） */
  arguments?: string;
  /** 执行状态 */
  status: "running" | "completed" | "failed";
  /** 执行结果（完成后） */
  result?: ToolExecutionResult;
  /** 开始时间 */
  startTime: Date;
  /** 结束时间（完成后） */
  endTime?: Date;
  /** 执行日志（实时更新） */
  logs?: string[];
}

/**
 * 解析流式事件
 * @param data - 原始事件数据
 * @returns 解析后的流式事件
 */
export function parseStreamEvent(data: unknown): StreamEvent | null {
  if (!data || typeof data !== "object") return null;

  const event = data as Record<string, unknown>;
  const type = event.type as string;

  switch (type) {
    case "text_delta":
      return {
        type: "text_delta",
        text: (event.text as string) || "",
      };
    case "reasoning_delta":
      return {
        type: "thinking_delta",
        text: (event.text as string) || "",
      };
    case "tool_start":
      return {
        type: "tool_start",
        tool_name: (event.tool_name as string) || "",
        tool_id: (event.tool_id as string) || "",
        arguments: event.arguments as string | undefined,
      };
    case "tool_end":
      return {
        type: "tool_end",
        tool_id: (event.tool_id as string) || "",
        result: event.result as ToolExecutionResult,
      };
    case "action_required":
      return {
        type: "action_required",
        request_id: (event.request_id as string) || "",
        action_type:
          (event.action_type as
            | "tool_confirmation"
            | "ask_user"
            | "elicitation") || "tool_confirmation",
        tool_name: event.tool_name as string | undefined,
        arguments: event.arguments as Record<string, unknown> | undefined,
        prompt: event.prompt as string | undefined,
        questions: event.questions as
          | Array<{
              question: string;
              header?: string;
              options?: Array<{
                label: string;
                description?: string;
              }>;
              multiSelect?: boolean;
            }>
          | undefined,
        requested_schema: event.requested_schema as
          | Record<string, unknown>
          | undefined,
      };
    case "done":
      return {
        type: "done",
        usage: event.usage as TokenUsage | undefined,
      };
    case "final_done":
      return {
        type: "final_done",
        usage: event.usage as TokenUsage | undefined,
      };
    case "error":
      return {
        type: "error",
        message: (event.message as string) || "Unknown error",
      };
    default:
      return null;
  }
}

/**
 * Agent 状态
 */
export interface AgentProcessStatus {
  running: boolean;
  base_url?: string;
  port?: number;
}

/**
 * 创建会话响应
 */
export interface CreateSessionResponse {
  session_id: string;
  credential_name: string;
  credential_uuid: string;
  provider_type: string;
  model?: string;
}

/**
 * 会话信息
 */
export interface SessionInfo {
  session_id: string;
  provider_type: string;
  model?: string;
  created_at: string;
  last_activity: string;
  messages_count: number;
}

/**
 * 图片输入
 */
export interface ImageInput {
  data: string;
  media_type: string;
}

/**
 * 启动 Agent（初始化原生 Agent）
 */
export async function startAgentProcess(): Promise<AgentProcessStatus> {
  return await safeInvoke("agent_start_process", {});
}

/**
 * 停止 Agent
 */
export async function stopAgentProcess(): Promise<void> {
  return await safeInvoke("agent_stop_process");
}

/**
 * 获取 Agent 状态
 */
export async function getAgentProcessStatus(): Promise<AgentProcessStatus> {
  return await safeInvoke("agent_get_process_status");
}

/**
 * Skill 信息
 */
export interface SkillInfo {
  name: string;
  description?: string;
  path?: string;
}

/**
 * 创建 Agent 会话
 */
export async function createAgentSession(
  providerType: string,
  model?: string,
  systemPrompt?: string,
  skills?: SkillInfo[],
): Promise<CreateSessionResponse> {
  return await safeInvoke("agent_create_session", {
    providerType,
    model,
    systemPrompt,
    skills,
  });
}

/**
 * 发送消息到 Agent（支持连续对话）- 非流式版本
 */
export async function sendAgentMessage(
  message: string,
  sessionId?: string,
  model?: string,
  images?: ImageInput[],
  webSearch?: boolean,
  thinking?: boolean,
): Promise<string> {
  return await safeInvoke("agent_send_message", {
    sessionId,
    message,
    images,
    model,
    webSearch,
    thinking,
  });
}

/**
 * 发送消息到 Agent（流式版本）
 *
 * 通过 Tauri 事件接收响应流，需要配合 listen() 使用：
 * @example
 * ```typescript
 * const unlisten = await listen<StreamEvent>(eventName, (event) => {
 *   const data = event.payload;
 *   if (data.type === "text_delta") {
 *     // 处理文本增量
 *   }
 * });
 * await sendAgentMessageStream(message, eventName, sessionId, model, undefined, provider);
 * ```
 */
export async function sendAgentMessageStream(
  message: string,
  eventName: string,
  sessionId?: string,
  model?: string,
  images?: ImageInput[],
  provider?: string,
  terminalMode?: boolean,
): Promise<void> {
  return await safeInvoke("native_agent_chat_stream", {
    message,
    eventName,
    sessionId,
    model,
    images,
    provider,
    terminalMode,
  });
}

/**
 * 获取会话列表
 */
export async function listAgentSessions(): Promise<SessionInfo[]> {
  return await safeInvoke("agent_list_sessions");
}

/**
 * 获取会话详情
 */
export async function getAgentSession(sessionId: string): Promise<SessionInfo> {
  return await safeInvoke("agent_get_session", {
    sessionId,
  });
}

/**
 * 删除会话
 */
export async function deleteAgentSession(sessionId: string): Promise<void> {
  return await safeInvoke("agent_delete_session", {
    sessionId,
  });
}

/**
 * Agent 消息内容类型
 */
export type AgentMessageContent =
  | string
  | Array<
      | { type: "text"; text: string }
      | { type: "image_url"; image_url: { url: string; detail?: string } }
    >;

/**
 * 工具调用
 */
export interface AgentToolCall {
  id: string;
  type: string;
  function: {
    name: string;
    arguments: string;
  };
}

/**
 * Agent 消息
 */
export interface AgentMessage {
  role: string;
  content: AgentMessageContent;
  timestamp: string;
  tool_calls?: AgentToolCall[];
  tool_call_id?: string;
}

/**
 * 获取会话消息列表
 */
export async function getAgentSessionMessages(
  sessionId: string,
): Promise<AgentMessage[]> {
  return await safeInvoke("agent_get_session_messages", {
    sessionId,
  });
}

// ============================================================
// aster Agent API (基于 aster 框架的完整 Agent 实现)
// ============================================================

/**
 * aster Agent 状态
 */
export interface asterAgentStatus {
  initialized: boolean;
  provider?: string;
  model?: string;
}

/**
 * aster Provider 信息
 */
export interface asterProviderInfo {
  name: string;
  display_name: string;
}

/**
 * aster 创建会话响应
 */
export interface asterCreateSessionResponse {
  session_id: string;
}

/**
 * 初始化 aster Agent
 *
 * @param providerName - Provider 名称 (如 "anthropic", "openai", "ollama")
 * @param modelName - 模型名称 (如 "claude-sonnet-4-20250514", "gpt-4o")
 */
export async function initasterAgent(
  providerName: string,
  modelName: string,
): Promise<asterAgentStatus> {
  return await safeInvoke("aster_agent_init", {
    providerName,
    modelName,
  });
}

/**
 * 获取 aster Agent 状态
 */
export async function getasterAgentStatus(): Promise<asterAgentStatus> {
  return await safeInvoke("aster_agent_status");
}

/**
 * 重置 aster Agent
 */
export async function resetasterAgent(): Promise<void> {
  return await safeInvoke("aster_agent_reset");
}

/**
 * 创建 aster Agent 会话
 */
export async function createasterSession(
  name?: string,
): Promise<asterCreateSessionResponse> {
  return await safeInvoke("aster_agent_create_session", { name });
}

/**
 * 发送消息到 aster Agent (流式响应)
 *
 * 通过 Tauri 事件接收响应流
 */
export async function sendasterMessage(
  sessionId: string,
  message: string,
  eventName: string,
): Promise<void> {
  return await safeInvoke("aster_agent_send_message", {
    request: {
      session_id: sessionId,
      message,
      event_name: eventName,
    },
  });
}

/**
 * 扩展 aster Agent 系统提示词
 */
export async function extendasterSystemPrompt(
  instruction: string,
): Promise<void> {
  return await safeInvoke("aster_agent_extend_system_prompt", { instruction });
}

/**
 * 获取 aster 支持的 Provider 列表
 */
export async function listasterProviders(): Promise<asterProviderInfo[]> {
  return await safeInvoke("aster_agent_list_providers");
}

// ============================================================
// Aster Agent API (基于 Aster 框架的 Agent 实现)
// ============================================================

/**
 * Aster Agent 状态
 */
export interface AsterAgentStatus {
  initialized: boolean;
  provider_configured: boolean;
  provider_name?: string;
  model_name?: string;
}

/**
 * Aster Provider 配置
 */
export interface AsterProviderConfig {
  provider_name: string;
  model_name: string;
  api_key?: string;
  base_url?: string;
}

/**
 * Aster 会话信息
 */
export interface AsterSessionInfo {
  id: string;
  name?: string;
  created_at: string;
  updated_at: string;
  messages_count: number;
}

/**
 * Aster 会话详情
 */
export interface AsterSessionDetail {
  id: string;
  name?: string;
  messages: Array<{
    role: string;
    content: string;
    timestamp: string;
  }>;
}

/**
 * 初始化 Aster Agent
 */
export async function initAsterAgent(): Promise<AsterAgentStatus> {
  return await safeInvoke("aster_agent_init");
}

/**
 * 获取 Aster Agent 状态
 */
export async function getAsterAgentStatus(): Promise<AsterAgentStatus> {
  return await safeInvoke("aster_agent_status");
}

/**
 * 配置 Aster Agent 的 Provider
 */
export async function configureAsterProvider(
  config: AsterProviderConfig,
  sessionId: string,
): Promise<AsterAgentStatus> {
  return await safeInvoke("aster_agent_configure_provider", {
    request: config,
    session_id: sessionId,
  });
}

/**
 * 发送消息到 Aster Agent (流式响应)
 *
 * 通过 Tauri 事件接收响应流
 */
export async function sendAsterMessageStream(
  message: string,
  sessionId: string,
  eventName: string,
  images?: ImageInput[],
  providerConfig?: AsterProviderConfig,
): Promise<void> {
  return await safeInvoke("aster_agent_chat_stream", {
    request: {
      message,
      session_id: sessionId,
      event_name: eventName,
      images,
      provider_config: providerConfig,
    },
  });
}

/**
 * 停止 Aster Agent 会话
 */
export async function stopAsterSession(sessionId: string): Promise<boolean> {
  return await safeInvoke("aster_agent_stop", { sessionId });
}

/**
 * 创建 Aster 会话
 */
export async function createAsterSession(
  workingDir?: string,
  name?: string,
): Promise<string> {
  return await safeInvoke("aster_session_create", { workingDir, name });
}

/**
 * 获取 Aster 会话列表
 */
export async function listAsterSessions(): Promise<AsterSessionInfo[]> {
  return await safeInvoke("aster_session_list");
}

/**
 * 获取 Aster 会话详情
 */
export async function getAsterSession(
  sessionId: string,
): Promise<AsterSessionDetail> {
  return await safeInvoke("aster_session_get", { sessionId });
}

/**
 * 确认 Aster Agent 权限请求
 */
export async function confirmAsterAction(
  requestId: string,
  confirmed: boolean,
  response?: string,
): Promise<void> {
  return await safeInvoke("aster_agent_confirm", {
    request: {
      request_id: requestId,
      confirmed,
      response,
    },
  });
}

// ============================================================
// Terminal Tool API (终端命令执行)
// ============================================================

/**
 * 终端命令请求（从后端发送到前端）
 */
export interface TerminalCommandRequest {
  /** 请求 ID */
  request_id: string;
  /** 要执行的命令 */
  command: string;
  /** 工作目录（可选） */
  working_dir?: string;
  /** 超时时间（秒） */
  timeout_secs: number;
}

/**
 * 终端命令响应（从前端发送到后端）
 */
export interface TerminalCommandResponse {
  /** 请求 ID */
  request_id: string;
  /** 是否成功 */
  success: boolean;
  /** 输出内容 */
  output: string;
  /** 错误信息 */
  error?: string;
  /** 退出码 */
  exit_code?: number;
  /** 是否被用户拒绝 */
  rejected: boolean;
}

/**
 * 发送终端命令响应到后端
 *
 * 当用户批准或拒绝命令后，调用此函数将结果发送给 TerminalTool
 */
export async function sendTerminalCommandResponse(
  response: TerminalCommandResponse,
): Promise<void> {
  return await safeInvoke("agent_terminal_command_response", {
    requestId: response.request_id,
    success: response.success,
    output: response.output,
    error: response.error,
    exitCode: response.exit_code,
    rejected: response.rejected,
  });
}

// ============================================================
// Terminal Scrollback Tool API (终端输出历史读取)
// ============================================================

/**
 * 终端滚动缓冲区请求（从后端发送到前端）
 */
export interface TermScrollbackRequest {
  /** 请求 ID */
  request_id: string;
  /** 终端会话 ID */
  session_id: string;
  /** 起始行号（可选，从 0 开始） */
  line_start?: number;
  /** 读取行数（可选） */
  count?: number;
}

/**
 * 终端滚动缓冲区响应（从前端发送到后端）
 */
export interface TermScrollbackResponse {
  /** 请求 ID */
  request_id: string;
  /** 是否成功 */
  success: boolean;
  /** 总行数 */
  total_lines: number;
  /** 实际返回的起始行号 */
  line_start: number;
  /** 实际返回的结束行号 */
  line_end: number;
  /** 输出内容 */
  content: string;
  /** 是否还有更多内容 */
  has_more: boolean;
  /** 错误信息 */
  error?: string;
}

/**
 * 发送终端滚动缓冲区响应到后端
 *
 * 当前端读取终端输出历史后，调用此函数将结果发送给 TermScrollbackTool
 */
export async function sendTermScrollbackResponse(
  response: TermScrollbackResponse,
): Promise<void> {
  return await safeInvoke("agent_term_scrollback_response", {
    requestId: response.request_id,
    success: response.success,
    totalLines: response.total_lines,
    lineStart: response.line_start,
    lineEnd: response.line_end,
    content: response.content,
    hasMore: response.has_more,
    error: response.error,
  });
}

// ============================================================
// Permission Confirmation API (权限确认)
// ============================================================

/**
 * 权限确认响应
 */
export interface PermissionResponse {
  /** 请求 ID */
  requestId: string;
  /** 是否确认 */
  confirmed: boolean;
  /** 响应内容（用户输入或选择的答案） */
  response?: string;
}

/**
 * 发送权限确认响应到后端
 *
 * 当用户确认或拒绝权限请求后，调用此函数将结果发送给 Agent
 */
export async function sendPermissionResponse(
  response: PermissionResponse,
): Promise<void> {
  return await safeInvoke("agent_permission_response", {
    requestId: response.requestId,
    confirmed: response.confirmed,
    response: response.response,
  });
}
