/**
 * @file useGeneralChatStore.ts
 * @description 通用对话功能的 Zustand 状态管理 Store
 * @module components/general-chat/store
 *
 * 实现了会话状态、消息状态、UI 状态和流式状态的管理
 * 使用 persist 中间件持久化 UI 布局状态
 *
 * @requirements 8.1, 8.2
 */

import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import type {
  Session,
  Message,
  UIState,
  StreamingState,
  CanvasState,
  MessageMetadata,
  ProviderSelectionState,
  ErrorInfo,
  PaginationState,
} from "../types";
import { ThreeStageWorkflowManager } from "@/lib/workflow/threeStageWorkflow";
import {
  DEFAULT_UI_STATE,
  DEFAULT_CANVAS_STATE,
  DEFAULT_STREAMING_STATE,
  DEFAULT_PROVIDER_SELECTION_STATE,
  DEFAULT_PAGINATION_STATE,
  parseApiError,
} from "../types";

// ============================================================================
// Store 状态接口
// ============================================================================

/**
 * GeneralChat Store 状态接口
 * @description 定义了 Store 中所有状态和操作方法
 */
export interface GeneralChatState {
  // ========== 会话状态 ==========
  /** 所有会话列表 */
  sessions: Session[];
  /** 当前选中的会话 ID */
  currentSessionId: string | null;

  // ========== 消息状态 ==========
  /** 消息映射表：sessionId -> messages */
  messages: Record<string, Message[]>;

  // ========== 流式状态 ==========
  /** 流式响应状态 */
  streaming: StreamingState;

  // ========== UI 状态 ==========
  /** UI 布局状态 */
  ui: UIState;

  // ========== 画布状态 ==========
  /** 画布面板状态 */
  canvas: CanvasState;

  // ========== Provider 选择状态 ==========
  /** Provider 和模型选择状态 */
  providerSelection: ProviderSelectionState;

  // ========== 分页状态 ==========
  /** 消息分页加载状态（按会话 ID 存储） */
  pagination: Record<string, PaginationState>;

  // ========== 三阶段工作流状态 ==========
  /** 工作流管理器实例（按会话 ID 存储） */
  workflowManagers: Record<string, ThreeStageWorkflowManager>;
  /** 工作流是否启用 */
  workflowEnabled: boolean;
  /** 工作流自动启用阈值（消息数量） */
  workflowThreshold: number;

  // ========== 会话操作 ==========
  /** 创建新会话 */
  createSession: () => Promise<string>;
  /** 选择/切换会话 */
  selectSession: (id: string) => void;
  /** 删除会话 */
  deleteSession: (id: string) => Promise<void>;
  /** 重命名会话 */
  renameSession: (id: string, name: string) => Promise<void>;
  /** 设置会话列表 */
  setSessions: (sessions: Session[]) => void;
  /** 更新单个会话 */
  updateSession: (id: string, updates: Partial<Session>) => void;

  // ========== 消息操作 ==========
  /** 发送消息 */
  sendMessage: (content: string, images?: File[]) => Promise<void>;
  /** 停止生成 */
  stopGeneration: () => void;
  /** 追加流式内容 */
  appendStreamingContent: (content: string) => void;
  /** 完成消息生成 */
  finalizeMessage: (metadata?: MessageMetadata) => void;
  /** 设置会话消息 */
  setMessages: (sessionId: string, messages: Message[]) => void;
  /** 添加消息 */
  addMessage: (message: Message) => void;
  /** 更新消息 */

  // ========== 三阶段工作流操作 ==========
  /** 初始化工作流 */
  initializeWorkflow: (
    sessionId: string,
    projectName: string,
    goal: string,
  ) => Promise<void>;
  /** 获取工作流管理器 */
  getWorkflowManager: (sessionId: string) => ThreeStageWorkflowManager | null;
  /** 启用/禁用工作流 */
  setWorkflowEnabled: (enabled: boolean) => void;
  /** 设置工作流阈值 */
  setWorkflowThreshold: (threshold: number) => void;
  /** 检查是否应该自动启用工作流 */
  shouldAutoEnableWorkflow: (sessionId: string) => boolean;
  updateMessage: (messageId: string, updates: Partial<Message>) => void;
  /** 开始流式响应 */
  startStreaming: (messageId: string) => void;
  /** 设置消息错误状态 */
  setMessageError: (messageId: string, error: ErrorInfo | string) => void;
  /** 重试发送消息 */
  retryMessage: (messageId: string) => Promise<void>;
  /** 清除消息错误 */
  clearMessageError: (messageId: string) => void;

  // ========== 分页操作 ==========
  /** 加载更多历史消息 */
  loadMoreMessages: (sessionId: string) => Promise<void>;
  /** 设置分页状态 */
  setPaginationState: (
    sessionId: string,
    state: Partial<PaginationState>,
  ) => void;
  /** 重置分页状态 */
  resetPagination: (sessionId: string) => void;
  /** 获取分页状态 */
  getPaginationState: (sessionId: string) => PaginationState;

  // ========== UI 操作 ==========
  /** 切换侧边栏折叠状态 */
  toggleSidebar: () => void;
  /** 设置侧边栏宽度 */
  setSidebarWidth: (width: number) => void;
  /** 切换画布折叠状态 */
  toggleCanvas: () => void;
  /** 设置画布宽度 */
  setCanvasWidth: (width: number) => void;

  // ========== 画布操作 ==========
  /** 打开画布并设置内容 */
  openCanvas: (state: Partial<CanvasState>) => void;
  /** 关闭画布 */
  closeCanvas: () => void;
  /** 更新画布内容 */
  updateCanvasContent: (content: string) => void;
  /** 设置画布编辑模式 */
  setCanvasEditing: (isEditing: boolean) => void;

  // ========== Provider 选择操作 ==========
  /** 设置选中的 Provider */
  setSelectedProvider: (providerKey: string | null) => void;
  /** 设置选中的模型 */
  setSelectedModel: (modelId: string | null) => void;
  /** 设置 Provider 加载状态 */
  setProviderLoading: (loading: boolean) => void;
  /** 设置 Provider 错误 */
  setProviderError: (error: string | null) => void;
  /** 重置 Provider 选择 */
  resetProviderSelection: () => void;

  // ========== 重置操作 ==========
  /** 重置 Store 到初始状态 */
  reset: () => void;
}

// ============================================================================
// 初始状态
// ============================================================================

const initialState = {
  // 会话状态
  sessions: [] as Session[],
  currentSessionId: null as string | null,

  // 消息状态
  messages: {} as Record<string, Message[]>,

  // 流式状态
  streaming: { ...DEFAULT_STREAMING_STATE },

  // UI 状态
  ui: { ...DEFAULT_UI_STATE },

  // 画布状态
  canvas: { ...DEFAULT_CANVAS_STATE },

  // Provider 选择状态
  providerSelection: { ...DEFAULT_PROVIDER_SELECTION_STATE },

  // 分页状态
  pagination: {} as Record<string, PaginationState>,

  // 三阶段工作流状态
  workflowManagers: {} as Record<string, ThreeStageWorkflowManager>,
  workflowEnabled: false,
  workflowThreshold: 5,
};

// ============================================================================
// 工具函数
// ============================================================================

/**
 * 生成唯一 ID
 */
const generateId = (): string => {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
};

/**
 * 获取当前时间戳
 */
const now = (): number => Date.now();

// ============================================================================
// Store 实现
// ============================================================================

/**
 * 通用对话 Zustand Store
 *
 * 使用 persist 中间件持久化 UI 状态和当前会话 ID
 * 会话和消息数据通过 Tauri 命令从 SQLite 数据库加载
 */
export const useGeneralChatStore = create<GeneralChatState>()(
  persist(
    (set, get) => ({
      ...initialState,

      // ========== 会话操作实现 ==========

      createSession: async () => {
        const id = generateId();
        const timestamp = now();
        const newSession: Session = {
          id,
          name: "新对话",
          createdAt: timestamp,
          updatedAt: timestamp,
          messageCount: 0,
        };

        set((state) => ({
          sessions: [newSession, ...state.sessions],
          currentSessionId: id,
          messages: {
            ...state.messages,
            [id]: [],
          },
        }));

        // TODO: 调用 Tauri 命令持久化到数据库
        // await invoke('general_chat_create_session', { name: newSession.name });

        return id;
      },

      selectSession: (id: string) => {
        const { sessions } = get();
        const sessionExists = sessions.some((s) => s.id === id);

        if (sessionExists) {
          set({ currentSessionId: id });
          // TODO: 如果消息未加载，调用 Tauri 命令加载消息
        }
      },

      deleteSession: async (id: string) => {
        const { sessions, currentSessionId, messages } = get();

        // 从列表中移除会话
        const newSessions = sessions.filter((s) => s.id !== id);

        // 移除关联的消息
        const newMessages = { ...messages };
        delete newMessages[id];

        // 如果删除的是当前会话，切换到第一个会话或设为 null
        const newCurrentId =
          currentSessionId === id
            ? newSessions.length > 0
              ? newSessions[0].id
              : null
            : currentSessionId;

        set({
          sessions: newSessions,
          currentSessionId: newCurrentId,
          messages: newMessages,
        });

        // TODO: 调用 Tauri 命令从数据库删除
        // await invoke('general_chat_delete_session', { sessionId: id });
      },

      renameSession: async (id: string, name: string) => {
        set((state) => ({
          sessions: state.sessions.map((s) =>
            s.id === id ? { ...s, name, updatedAt: now() } : s,
          ),
        }));

        // TODO: 调用 Tauri 命令持久化到数据库
        // await invoke('general_chat_rename_session', { sessionId: id, name });
      },

      setSessions: (sessions: Session[]) => {
        set({ sessions });
      },

      updateSession: (id: string, updates: Partial<Session>) => {
        set((state) => ({
          sessions: state.sessions.map((s) =>
            s.id === id ? { ...s, ...updates, updatedAt: now() } : s,
          ),
        }));
      },

      // ========== 消息操作实现 ==========

      sendMessage: async (content: string, images?: File[]) => {
        const { currentSessionId, messages } = get();

        // 验证：空白消息且无图片不发送
        if (!content.trim() && (!images || images.length === 0)) {
          return;
        }

        // 验证：必须有当前会话
        if (!currentSessionId) {
          console.warn("No current session selected");
          return;
        }

        const messageId = generateId();
        const timestamp = now();

        // 处理图片数据
        let imageData: Array<{ data: string; media_type: string }> | undefined;
        if (images && images.length > 0) {
          try {
            // 将 File 对象转换为 base64 格式
            imageData = await Promise.all(
              images.map(async (file) => {
                return new Promise<{ data: string; media_type: string }>(
                  (resolve, reject) => {
                    const reader = new FileReader();
                    reader.onload = (e) => {
                      const result = e.target?.result as string;
                      if (!result) {
                        reject(new Error("无法读取文件"));
                        return;
                      }
                      // 提取 base64 数据（去掉 data:image/xxx;base64, 前缀）
                      const base64Data = result.split(",")[1];
                      resolve({
                        data: base64Data,
                        media_type: file.type,
                      });
                    };
                    reader.onerror = () => reject(new Error("文件读取失败"));
                    reader.readAsDataURL(file);
                  },
                );
              }),
            );
          } catch (error) {
            console.error("图片处理失败:", error);
            return;
          }
        }

        // 创建用户消息
        const userMessage: Message = {
          id: messageId,
          sessionId: currentSessionId,
          role: "user",
          content: content.trim() || "[图片]",
          blocks: [
            ...(content.trim()
              ? [{ type: "text" as const, content: content.trim() }]
              : []),
            ...(imageData?.map((img) => ({
              type: "image" as const,
              content: `data:${img.media_type};base64,${img.data}`,
              mimeType: img.media_type,
            })) || []),
          ],
          status: "complete",
          createdAt: timestamp,
        };

        // 添加用户消息到列表
        const currentMessages = messages[currentSessionId] || [];
        set((state) => ({
          messages: {
            ...state.messages,
            [currentSessionId]: [...currentMessages, userMessage],
          },
        }));

        // 更新会话的消息数量和更新时间
        get().updateSession(currentSessionId, {
          messageCount: currentMessages.length + 1,
        });

        // 创建 AI 响应消息占位符
        const assistantMessageId = generateId();
        const assistantMessage: Message = {
          id: assistantMessageId,
          sessionId: currentSessionId,
          role: "assistant",
          content: "",
          blocks: [],
          status: "pending",
          createdAt: timestamp + 1,
        };

        // 添加 AI 消息占位符
        set((state) => ({
          messages: {
            ...state.messages,
            [currentSessionId]: [
              ...(state.messages[currentSessionId] || []),
              assistantMessage,
            ],
          },
        }));

        // 开始流式响应
        get().startStreaming(assistantMessageId);

        // 三阶段工作流集成
        const {
          workflowEnabled,
          workflowManagers,
          messages: allMessages,
        } = get();
        let workflowManager = workflowManagers[currentSessionId];

        // 检查是否应该自动启用工作流
        if (
          !workflowManager &&
          get().shouldAutoEnableWorkflow(currentSessionId)
        ) {
          // 自动初始化工作流
          await get().initializeWorkflow(
            currentSessionId,
            "智能对话任务",
            "协助用户完成复杂的对话任务，提供准确和有用的回答",
          );
          workflowManager = get().workflowManagers[currentSessionId];
        }

        // 如果启用了工作流，执行 Pre-Action 阶段
        if (workflowManager && workflowEnabled) {
          try {
            const messageCount = (allMessages[currentSessionId] || []).length;
            const actionContext = {
              sessionId: currentSessionId,
              actionType: "send_message",
              actionDescription: `发送消息: ${content.substring(0, 100)}${content.length > 100 ? "..." : ""}`,
              toolName: "aster_agent_chat_stream",
              toolParameters: {
                message: content.trim() || "请分析这张图片",
                hasImages: imageData ? "true" : "false",
              },
              messageCount,
            };

            // Pre-Action: 上下文刷新
            await workflowManager.preAction(actionContext);
          } catch (error) {
            console.warn("工作流 Pre-Action 执行失败:", error);
          }
        }

        try {
          // 调用 Tauri 命令发送消息并开始流式响应
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("aster_agent_chat_stream", {
            sessionId: currentSessionId,
            message: content.trim() || "请分析这张图片",
            eventName: `general-chat-stream-${currentSessionId}`,
            images: imageData,
          });

          // 如果启用了工作流，执行 Action 阶段
          if (workflowManager && workflowEnabled) {
            try {
              const messageCount = (get().messages[currentSessionId] || [])
                .length;
              const actionContext = {
                sessionId: currentSessionId,
                actionType: "send_message",
                actionDescription: `发送消息: ${content.substring(0, 100)}${content.length > 100 ? "..." : ""}`,
                toolName: "aster_agent_chat_stream",
                messageCount,
              };

              await workflowManager.executeAction(
                actionContext,
                "消息发送成功，等待 AI 响应",
              );
            } catch (error) {
              console.warn("工作流 Action 执行失败:", error);
            }
          }
        } catch (error) {
          console.error("发送消息失败:", error);

          // 如果启用了工作流，执行 Post-Action 阶段（错误情况）
          if (workflowManager && workflowEnabled) {
            try {
              const messageCount = (get().messages[currentSessionId] || [])
                .length;
              const actionContext = {
                sessionId: currentSessionId,
                actionType: "send_message",
                actionDescription: `发送消息: ${content.substring(0, 100)}${content.length > 100 ? "..." : ""}`,
                messageCount,
              };

              await workflowManager.postAction(
                actionContext,
                "",
                error as string,
              );
            } catch (workflowError) {
              console.warn("工作流 Post-Action 执行失败:", workflowError);
            }
          }

          // 设置错误状态
          get().setMessageError(assistantMessageId, error as string);
        }
      },

      stopGeneration: () => {
        const { streaming, currentSessionId, messages } = get();

        if (!streaming.isStreaming || !streaming.currentMessageId) {
          return;
        }

        // 将当前流式消息标记为完成
        if (currentSessionId) {
          const currentMessages = messages[currentSessionId] || [];
          const updatedMessages = currentMessages.map((m) =>
            m.id === streaming.currentMessageId
              ? {
                  ...m,
                  status: "complete" as const,
                  content: streaming.partialContent,
                }
              : m,
          );

          set((state) => ({
            messages: {
              ...state.messages,
              [currentSessionId]: updatedMessages,
            },
            streaming: { ...DEFAULT_STREAMING_STATE },
          }));
        } else {
          set({ streaming: { ...DEFAULT_STREAMING_STATE } });
        }

        // TODO: 调用 Tauri 命令停止生成
        // await invoke('general_chat_stop_generation', { sessionId: currentSessionId });
      },

      appendStreamingContent: (content: string) => {
        set((state) => ({
          streaming: {
            ...state.streaming,
            partialContent: state.streaming.partialContent + content,
          },
        }));

        // 同时更新消息内容
        const { streaming, currentSessionId, messages } = get();
        if (streaming.currentMessageId && currentSessionId) {
          const currentMessages = messages[currentSessionId] || [];
          const updatedMessages = currentMessages.map((m) =>
            m.id === streaming.currentMessageId
              ? { ...m, content: get().streaming.partialContent }
              : m,
          );

          set((state) => ({
            messages: {
              ...state.messages,
              [currentSessionId]: updatedMessages,
            },
          }));
        }
      },

      finalizeMessage: async (metadata?: MessageMetadata) => {
        const {
          streaming,
          currentSessionId,
          messages,
          workflowManagers,
          workflowEnabled,
        } = get();

        if (!streaming.currentMessageId || !currentSessionId) {
          return;
        }

        const currentMessages = messages[currentSessionId] || [];
        const updatedMessages = currentMessages.map((m) =>
          m.id === streaming.currentMessageId
            ? {
                ...m,
                status: "complete" as const,
                content: streaming.partialContent,
                metadata,
              }
            : m,
        );

        set((state) => ({
          messages: {
            ...state.messages,
            [currentSessionId]: updatedMessages,
          },
          streaming: { ...DEFAULT_STREAMING_STATE },
        }));

        // 如果启用了工作流，执行 Post-Action 阶段
        const workflowManager = workflowManagers[currentSessionId];
        if (workflowManager && workflowEnabled) {
          try {
            const messageCount = updatedMessages.length;
            const actionContext = {
              sessionId: currentSessionId,
              actionType: "receive_message",
              actionDescription: `接收 AI 响应: ${streaming.partialContent.substring(0, 100)}${streaming.partialContent.length > 100 ? "..." : ""}`,
              messageCount,
            };

            await workflowManager.postAction(
              actionContext,
              streaming.partialContent,
            );
          } catch (error) {
            console.warn("工作流 Post-Action 执行失败:", error);
          }
        }
      },

      setMessages: (sessionId: string, newMessages: Message[]) => {
        set((state) => ({
          messages: {
            ...state.messages,
            [sessionId]: newMessages,
          },
        }));
      },

      addMessage: (message: Message) => {
        const { sessionId } = message;
        set((state) => ({
          messages: {
            ...state.messages,
            [sessionId]: [...(state.messages[sessionId] || []), message],
          },
        }));
      },

      updateMessage: (messageId: string, updates: Partial<Message>) => {
        const { currentSessionId, messages } = get();
        if (!currentSessionId) return;

        const currentMessages = messages[currentSessionId] || [];
        const updatedMessages = currentMessages.map((m) =>
          m.id === messageId ? { ...m, ...updates } : m,
        );

        set((state) => ({
          messages: {
            ...state.messages,
            [currentSessionId]: updatedMessages,
          },
        }));
      },

      startStreaming: (messageId: string) => {
        set({
          streaming: {
            isStreaming: true,
            currentMessageId: messageId,
            partialContent: "",
          },
        });
      },

      setMessageError: (messageId: string, error: ErrorInfo | string) => {
        const { currentSessionId, messages } = get();
        if (!currentSessionId) return;

        // 如果传入的是字符串，解析为 ErrorInfo
        const errorInfo: ErrorInfo =
          typeof error === "string" ? parseApiError(error) : error;

        const currentMessages = messages[currentSessionId] || [];
        const updatedMessages = currentMessages.map((m) =>
          m.id === messageId
            ? { ...m, status: "error" as const, error: errorInfo }
            : m,
        );

        set((state) => ({
          messages: {
            ...state.messages,
            [currentSessionId]: updatedMessages,
          },
          streaming: { ...DEFAULT_STREAMING_STATE },
        }));
      },

      retryMessage: async (messageId: string) => {
        const { currentSessionId, messages } = get();
        if (!currentSessionId) return;

        const currentMessages = messages[currentSessionId] || [];
        const errorMessage = currentMessages.find((m) => m.id === messageId);

        if (!errorMessage || errorMessage.status !== "error") {
          return;
        }

        // 找到错误消息之前的用户消息
        const messageIndex = currentMessages.findIndex(
          (m) => m.id === messageId,
        );
        if (messageIndex <= 0) return;

        // 查找最近的用户消息
        let userMessage: Message | null = null;
        for (let i = messageIndex - 1; i >= 0; i--) {
          if (currentMessages[i].role === "user") {
            userMessage = currentMessages[i];
            break;
          }
        }

        if (!userMessage) return;

        // 清除错误状态，将消息状态改为 pending
        const updatedMessages = currentMessages.map((m) =>
          m.id === messageId
            ? {
                ...m,
                status: "pending" as const,
                error: undefined,
                content: "",
              }
            : m,
        );

        set((state) => ({
          messages: {
            ...state.messages,
            [currentSessionId]: updatedMessages,
          },
        }));

        // TODO: 调用 Tauri 命令重新发送消息
        // await invoke('general_chat_send_message', {
        //   sessionId: currentSessionId,
        //   content: userMessage.content,
        //   eventName: `chat-stream-${currentSessionId}`,
        // });
      },

      clearMessageError: (messageId: string) => {
        const { currentSessionId, messages } = get();
        if (!currentSessionId) return;

        const currentMessages = messages[currentSessionId] || [];
        const updatedMessages = currentMessages.map((m) =>
          m.id === messageId ? { ...m, error: undefined } : m,
        );

        set((state) => ({
          messages: {
            ...state.messages,
            [currentSessionId]: updatedMessages,
          },
        }));
      },

      // ========== 分页操作实现 ==========

      loadMoreMessages: async (sessionId: string) => {
        const { messages, pagination } = get();
        const currentPagination = pagination[sessionId] || {
          ...DEFAULT_PAGINATION_STATE,
        };

        // 如果正在加载或没有更多消息，直接返回
        if (
          currentPagination.isLoadingMore ||
          !currentPagination.hasMoreMessages
        ) {
          return;
        }

        // 设置加载状态
        set((state) => ({
          pagination: {
            ...state.pagination,
            [sessionId]: {
              ...currentPagination,
              isLoadingMore: true,
            },
          },
        }));

        try {
          // 获取当前会话的消息列表
          const currentMessages = messages[sessionId] || [];

          // 获取最早消息的 ID 用于分页
          const oldestMessage =
            currentMessages.length > 0 ? currentMessages[0] : null;
          const beforeId = oldestMessage?.id || null;

          // 调用 Tauri 命令获取更多消息
          const { invoke } = await import("@tauri-apps/api/core");
          const olderMessages = await invoke<
            Array<{
              id: string;
              session_id: string;
              role: string;
              content: string;
              blocks: Array<{
                type: string;
                content: string;
                language?: string;
                filename?: string;
                mime_type?: string;
              }> | null;
              status: string;
              created_at: number;
              metadata: Record<string, unknown> | null;
            }>
          >("general_chat_get_messages", {
            sessionId,
            limit: currentPagination.pageSize,
            beforeId,
          });

          // 转换后端消息格式为前端格式
          const convertedMessages: Message[] = olderMessages.map((msg) => ({
            id: msg.id,
            sessionId: msg.session_id,
            role: msg.role as Message["role"],
            content: msg.content,
            blocks: msg.blocks?.map((b) => ({
              type: b.type as Message["blocks"][0]["type"],
              content: b.content,
              language: b.language,
              filename: b.filename,
              mimeType: b.mime_type,
            })) || [{ type: "text" as const, content: msg.content }],
            status: msg.status as Message["status"],
            createdAt: msg.created_at,
            metadata: msg.metadata
              ? {
                  model: msg.metadata.model as string | undefined,
                  tokens: msg.metadata.tokens as number | undefined,
                  duration: msg.metadata.duration as number | undefined,
                }
              : undefined,
          }));

          // 判断是否还有更多消息
          const hasMore =
            convertedMessages.length >= currentPagination.pageSize;

          // 将旧消息添加到列表前面
          set((state) => ({
            messages: {
              ...state.messages,
              [sessionId]: [
                ...convertedMessages,
                ...(state.messages[sessionId] || []),
              ],
            },
            pagination: {
              ...state.pagination,
              [sessionId]: {
                ...currentPagination,
                isLoadingMore: false,
                hasMoreMessages: hasMore,
                oldestMessageId:
                  convertedMessages.length > 0
                    ? convertedMessages[0].id
                    : currentPagination.oldestMessageId,
              },
            },
          }));
        } catch (error) {
          console.error("加载更多消息失败:", error);
          // 加载失败时重置加载状态
          set((state) => ({
            pagination: {
              ...state.pagination,
              [sessionId]: {
                ...currentPagination,
                isLoadingMore: false,
              },
            },
          }));
        }
      },

      setPaginationState: (
        sessionId: string,
        state: Partial<PaginationState>,
      ) => {
        set((prev) => ({
          pagination: {
            ...prev.pagination,
            [sessionId]: {
              ...(prev.pagination[sessionId] || {
                ...DEFAULT_PAGINATION_STATE,
              }),
              ...state,
            },
          },
        }));
      },

      resetPagination: (sessionId: string) => {
        set((state) => ({
          pagination: {
            ...state.pagination,
            [sessionId]: { ...DEFAULT_PAGINATION_STATE },
          },
        }));
      },

      getPaginationState: (sessionId: string) => {
        const { pagination } = get();
        return pagination[sessionId] || { ...DEFAULT_PAGINATION_STATE };
      },

      // ========== UI 操作实现 ==========

      toggleSidebar: () => {
        set((state) => ({
          ui: {
            ...state.ui,
            sidebarCollapsed: !state.ui.sidebarCollapsed,
          },
        }));
      },

      setSidebarWidth: (width: number) => {
        set((state) => ({
          ui: {
            ...state.ui,
            sidebarWidth: Math.max(200, Math.min(400, width)), // 限制宽度范围
          },
        }));
      },

      toggleCanvas: () => {
        set((state) => ({
          ui: {
            ...state.ui,
            canvasCollapsed: !state.ui.canvasCollapsed,
          },
          canvas: state.ui.canvasCollapsed
            ? { ...state.canvas, isOpen: true }
            : { ...state.canvas, isOpen: false },
        }));
      },

      setCanvasWidth: (width: number) => {
        set((state) => ({
          ui: {
            ...state.ui,
            canvasWidth: Math.max(300, Math.min(800, width)), // 限制宽度范围
          },
        }));
      },

      // ========== 画布操作实现 ==========

      openCanvas: (canvasState: Partial<CanvasState>) => {
        set((state) => ({
          canvas: {
            ...state.canvas,
            ...canvasState,
            isOpen: true,
          },
          ui: {
            ...state.ui,
            canvasCollapsed: false,
          },
        }));
      },

      closeCanvas: () => {
        set((state) => ({
          canvas: {
            ...DEFAULT_CANVAS_STATE,
          },
          ui: {
            ...state.ui,
            canvasCollapsed: true,
          },
        }));
      },

      updateCanvasContent: (content: string) => {
        set((state) => ({
          canvas: {
            ...state.canvas,
            content,
          },
        }));
      },

      setCanvasEditing: (isEditing: boolean) => {
        set((state) => ({
          canvas: {
            ...state.canvas,
            isEditing,
          },
        }));
      },

      // ========== Provider 选择操作实现 ==========

      setSelectedProvider: (providerKey: string | null) => {
        set((state) => ({
          providerSelection: {
            ...state.providerSelection,
            selectedProviderKey: providerKey,
            // 切换 Provider 时清除模型选择
            selectedModelId: null,
          },
        }));
      },

      setSelectedModel: (modelId: string | null) => {
        set((state) => ({
          providerSelection: {
            ...state.providerSelection,
            selectedModelId: modelId,
          },
        }));
      },

      setProviderLoading: (loading: boolean) => {
        set((state) => ({
          providerSelection: {
            ...state.providerSelection,
            isLoadingProviders: loading,
          },
        }));
      },

      setProviderError: (error: string | null) => {
        set((state) => ({
          providerSelection: {
            ...state.providerSelection,
            providerError: error,
          },
        }));
      },

      resetProviderSelection: () => {
        set({
          providerSelection: { ...DEFAULT_PROVIDER_SELECTION_STATE },
        });
      },

      // ========== 重置操作 ==========

      reset: () => {
        set(initialState);
      },

      // ========== 三阶段工作流操作实现 ==========

      initializeWorkflow: async (
        sessionId: string,
        projectName: string,
        goal: string,
      ) => {
        const { workflowManagers } = get();

        // 如果已存在工作流管理器，直接返回
        if (workflowManagers[sessionId]) {
          return;
        }

        // 创建新的工作流管理器
        const workflowManager = new ThreeStageWorkflowManager(sessionId);

        // 初始化工作流配置
        const workflowConfig = {
          sessionId,
          projectName,
          goal,
          phases: [
            {
              number: 1,
              name: "需求理解与发现",
              status: "in_progress" as const,
              tasks: [
                "理解用户意图和需求",
                "识别约束条件和依赖关系",
                "记录发现到 findings.md",
              ],
            },
            {
              number: 2,
              name: "规划与结构设计",
              status: "pending" as const,
              tasks: [
                "定义技术方案和架构",
                "创建项目结构（如需要）",
                "记录关键决策和理由",
              ],
            },
            {
              number: 3,
              name: "实施执行",
              status: "pending" as const,
              tasks: [
                "按计划逐步执行",
                "执行前先写入文件",
                "增量测试并记录结果",
              ],
            },
            {
              number: 4,
              name: "测试与验证",
              status: "pending" as const,
              tasks: [
                "验证所有需求已满足",
                "记录测试结果到 progress.md",
                "修复发现的问题并记录解决方案",
              ],
            },
            {
              number: 5,
              name: "交付与完成",
              status: "pending" as const,
              tasks: [
                "审查所有输出文件和交付物",
                "确保完整性和质量",
                "向用户交付最终结果",
              ],
            },
          ],
        };

        try {
          // 初始化工作流
          await workflowManager.initializeWorkflow(workflowConfig);

          // 保存到状态中
          set((state) => ({
            workflowManagers: {
              ...state.workflowManagers,
              [sessionId]: workflowManager,
            },
          }));

          console.log(`三阶段工作流已为会话 ${sessionId} 初始化`);
        } catch (error) {
          console.error("工作流初始化失败:", error);
          throw error;
        }
      },

      getWorkflowManager: (sessionId: string) => {
        const { workflowManagers } = get();
        return workflowManagers[sessionId] || null;
      },

      setWorkflowEnabled: (enabled: boolean) => {
        set({ workflowEnabled: enabled });
      },

      setWorkflowThreshold: (threshold: number) => {
        set({ workflowThreshold: threshold });
      },

      shouldAutoEnableWorkflow: (sessionId: string) => {
        const { messages, workflowThreshold, workflowEnabled } = get();
        if (!workflowEnabled) return false;

        const messageCount = (messages[sessionId] || []).length;
        return messageCount >= workflowThreshold;
      },
    }),
    {
      name: "general-chat-storage",
      storage: createJSONStorage(() => localStorage),
      // 只持久化 UI 状态、当前会话 ID 和 Provider 选择
      partialize: (state) => ({
        ui: state.ui,
        currentSessionId: state.currentSessionId,
        providerSelection: {
          selectedProviderKey: state.providerSelection.selectedProviderKey,
          selectedModelId: state.providerSelection.selectedModelId,
        },
      }),
    },
  ),
);

// ============================================================================
// 选择器 Hooks（用于性能优化）
// ============================================================================

/**
 * 获取当前会话
 */
export const useCurrentSession = () =>
  useGeneralChatStore((state) => {
    const { sessions, currentSessionId } = state;
    return sessions.find((s) => s.id === currentSessionId) || null;
  });

/**
 * 获取当前会话的消息列表
 */
export const useCurrentMessages = () =>
  useGeneralChatStore((state) => {
    const { messages, currentSessionId } = state;
    return currentSessionId ? messages[currentSessionId] || [] : [];
  });

/**
 * 获取流式状态
 */
export const useStreamingState = () =>
  useGeneralChatStore((state) => state.streaming);

/**
 * 获取 UI 状态
 */
export const useUIState = () => useGeneralChatStore((state) => state.ui);

/**
 * 获取画布状态
 */
export const useCanvasState = () =>
  useGeneralChatStore((state) => state.canvas);

/**
 * 获取会话列表
 */
export const useSessions = () => useGeneralChatStore((state) => state.sessions);

/**
 * 检查是否正在流式生成
 */
export const useIsStreaming = () =>
  useGeneralChatStore((state) => state.streaming.isStreaming);

/**
 * 获取 Provider 选择状态
 */
export const useProviderSelection = () =>
  useGeneralChatStore((state) => state.providerSelection);

/**
 * 获取选中的 Provider Key
 */
export const useSelectedProviderKey = () =>
  useGeneralChatStore((state) => state.providerSelection.selectedProviderKey);

/**
 * 获取选中的模型 ID
 */
export const useSelectedModelId = () =>
  useGeneralChatStore((state) => state.providerSelection.selectedModelId);

/**
 * 获取当前会话的分页状态
 * @requirements 10.2
 */
export const useCurrentPagination = () =>
  useGeneralChatStore((state) => {
    const { pagination, currentSessionId } = state;
    return currentSessionId
      ? pagination[currentSessionId] || { ...DEFAULT_PAGINATION_STATE }
      : { ...DEFAULT_PAGINATION_STATE };
  });

/**
 * 获取指定会话的分页状态
 * @requirements 10.2
 */
export const usePaginationState = (sessionId: string | null) =>
  useGeneralChatStore((state) => {
    if (!sessionId) return { ...DEFAULT_PAGINATION_STATE };
    return state.pagination[sessionId] || { ...DEFAULT_PAGINATION_STATE };
  });

/**
 * 检查是否正在加载更多消息
 * @requirements 10.2
 */
export const useIsLoadingMore = () =>
  useGeneralChatStore((state) => {
    const { pagination, currentSessionId } = state;
    if (!currentSessionId) return false;
    return pagination[currentSessionId]?.isLoadingMore || false;
  });

/**
 * 检查是否还有更多消息可加载
 * @requirements 10.2
 */
export const useHasMoreMessages = () =>
  useGeneralChatStore((state) => {
    const { pagination, currentSessionId } = state;
    if (!currentSessionId) return true;
    return pagination[currentSessionId]?.hasMoreMessages ?? true;
  });
