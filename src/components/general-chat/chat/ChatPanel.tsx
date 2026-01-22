/**
 * @file ChatPanel.tsx
 * @description 聊天面板组件 - 显示消息列表和输入栏
 * @module components/general-chat/chat/ChatPanel
 *
 * @requirements 3.1, 3.6, 5.4, 5.5, 2.6, 9.5, 10.2
 */

import React, { useState, useMemo, useCallback } from "react";
import { Settings, AlertTriangle, Loader2 } from "lucide-react";
import { useGeneralChatStore } from "../store/useGeneralChatStore";
import type { CanvasState, ContentBlock } from "../types";
import { DEFAULT_PAGINATION_STATE } from "../types";
import { MessageList } from "./MessageList";
import { Inputbar } from "@/components/agent/chat/components/Inputbar";
import { CompactModelSelector } from "./CompactModelSelector";
import { WorkflowStatusPanel } from "../components/WorkflowStatusPanel";
import { useConfiguredProviders } from "@/hooks/useConfiguredProviders";
import type { MessageImage } from "@/components/agent/chat/types";

interface ChatPanelProps {
  /** 当前会话 ID */
  sessionId: string | null;
  /** 打开画布回调 */
  onOpenCanvas: (state: CanvasState) => void;
  /** 页面导航回调 */
  onNavigate?: (page: string) => void;
}

/**
 * 空状态组件
 */
const EmptyState = () => (
  <div className="flex flex-col items-center justify-center h-full text-center px-8">
    <div className="w-16 h-16 mb-4 rounded-full bg-accent/10 flex items-center justify-center">
      <svg
        className="w-8 h-8 text-accent"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={1.5}
          d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
        />
      </svg>
    </div>
    <h3 className="text-lg font-medium text-ink-900 mb-2">开始新对话</h3>
    <p className="text-sm text-ink-500 max-w-sm">
      选择一个会话或创建新会话，开始与 AI 助手对话
    </p>
  </div>
);

// ============================================================================
// 无 Provider 提示组件
// ============================================================================

interface NoProviderPromptProps {
  /** 导航到配置页面的回调 */
  onNavigateToConfig?: () => void;
}

/**
 * 无 Provider 提示组件
 */
const NoProviderPrompt: React.FC<NoProviderPromptProps> = ({
  onNavigateToConfig,
}) => (
  <div className="flex flex-col items-center justify-center h-full text-center px-8">
    <div className="w-20 h-20 mb-6 rounded-full bg-amber-100 flex items-center justify-center">
      <AlertTriangle className="w-10 h-10 text-amber-600" />
    </div>
    <h3 className="text-xl font-semibold text-ink-900 mb-3">
      尚未配置 AI Provider
    </h3>
    <p className="text-sm text-ink-500 max-w-md mb-6 leading-relaxed">
      要开始使用 AI 对话功能，您需要先配置至少一个 Provider 凭证。 支持
      Kiro、Gemini、OpenAI、Claude 等多种 Provider。
    </p>
    {onNavigateToConfig && (
      <button
        type="button"
        onClick={onNavigateToConfig}
        className="inline-flex items-center gap-2 px-6 py-3 bg-accent text-white rounded-lg font-medium hover:bg-accent/90 transition-colors shadow-sm"
      >
        <Settings className="w-5 h-5" />
        前往配置 Provider
      </button>
    )}
    <p className="text-xs text-ink-400 mt-4">
      配置完成后，返回此页面即可开始对话
    </p>
  </div>
);

/**
 * 聊天面板组件
 */
export const ChatPanel: React.FC<ChatPanelProps> = ({
  sessionId,
  onOpenCanvas,
  onNavigate,
}) => {
  const [input, setInput] = useState("");
  const [retryingMessageId, setRetryingMessageId] = useState<string | null>(
    null,
  );

  // 直接从 store 获取状态
  const messages = useGeneralChatStore((state) => state.messages);
  const streaming = useGeneralChatStore((state) => state.streaming);
  const pagination = useGeneralChatStore((state) => state.pagination);
  const workflowEnabled = useGeneralChatStore((state) => state.workflowEnabled);
  const sendMessage = useGeneralChatStore((state) => state.sendMessage);
  const stopGeneration = useGeneralChatStore((state) => state.stopGeneration);
  const retryMessage = useGeneralChatStore((state) => state.retryMessage);
  const loadMoreMessages = useGeneralChatStore(
    (state) => state.loadMoreMessages,
  );
  const initializeWorkflow = useGeneralChatStore(
    (state) => state.initializeWorkflow,
  );
  const getWorkflowManager = useGeneralChatStore(
    (state) => state.getWorkflowManager,
  );

  // 获取分页状态
  const paginationState = sessionId
    ? pagination[sessionId] || DEFAULT_PAGINATION_STATE
    : DEFAULT_PAGINATION_STATE;
  const { hasMoreMessages, isLoadingMore } = paginationState;

  // 直接使用 useConfiguredProviders 检查是否有可用的 Provider
  const { providers, loading: isProviderLoading } = useConfiguredProviders();
  const hasAvailableProvider = providers.length > 0;

  // 从 streaming 对象中解构状态
  const { isStreaming, partialContent } = streaming;

  // 获取当前会话的消息
  const currentMessages = useMemo(
    () => (sessionId ? messages[sessionId] || [] : []),
    [sessionId, messages],
  );

  // 获取工作流状态
  const workflowManager = sessionId ? getWorkflowManager(sessionId) : null;
  const isWorkflowInitialized = workflowManager !== null;
  const messageCount = currentMessages.length;
  const visualOperationCount = currentMessages.filter(
    (m) => m.images && m.images.length > 0,
  ).length;

  // 处理加载更多消息
  const handleLoadMore = useCallback(() => {
    if (sessionId && hasMoreMessages && !isLoadingMore) {
      loadMoreMessages(sessionId);
    }
  }, [sessionId, hasMoreMessages, isLoadingMore, loadMoreMessages]);

  // 处理初始化工作流
  const handleInitializeWorkflow = useCallback(
    async (projectName: string, goal: string) => {
      if (sessionId) {
        await initializeWorkflow(sessionId, projectName, goal);
      }
    },
    [sessionId, initializeWorkflow],
  );

  // 处理结束工作流
  const handleFinalizeWorkflow = useCallback(async () => {
    if (sessionId && workflowManager) {
      try {
        await workflowManager.finalizeWorkflow();
      } catch (error) {
        console.error("结束工作流失败:", error);
      }
    }
  }, [sessionId, workflowManager]);

  // 处理导航到 Provider 配置页面
  const handleNavigateToProviderConfig = useCallback(() => {
    onNavigate?.("provider-pool");
  }, [onNavigate]);

  // 处理发送消息
  const handleSend = useCallback(
    async (
      images?: MessageImage[],
      _webSearch?: boolean,
      _thinking?: boolean,
    ) => {
      if (!sessionId || (!input.trim() && (!images || images.length === 0)))
        return;

      const content = input.trim();
      setInput("");

      // 将 MessageImage 转换为 File 对象（用于 store）
      let files: File[] | undefined;
      if (images && images.length > 0) {
        files = images.map((img, index) => {
          // 将 base64 转换回 Blob，然后创建 File
          const byteCharacters = atob(img.data);
          const byteNumbers = new Array(byteCharacters.length);
          for (let i = 0; i < byteCharacters.length; i++) {
            byteNumbers[i] = byteCharacters.charCodeAt(i);
          }
          const byteArray = new Uint8Array(byteNumbers);
          const blob = new Blob([byteArray], { type: img.mediaType });
          return new File(
            [blob],
            `image_${index}.${img.mediaType.split("/")[1]}`,
            { type: img.mediaType },
          );
        });
      }

      await sendMessage(content || "请分析这张图片", files);
    },
    [sessionId, input, sendMessage],
  );

  // 处理停止生成
  const handleStop = useCallback(() => {
    stopGeneration();
  }, [stopGeneration]);

  // 处理复制内容
  const handleCopy = useCallback((content: string) => {
    navigator.clipboard.writeText(content);
  }, []);

  // 处理在画布中打开
  const handleOpenInCanvas = useCallback(
    (block: ContentBlock) => {
      onOpenCanvas({
        isOpen: true,
        contentType: block.type === "code" ? "code" : "markdown",
        content: block.content,
        language: block.language,
        filename: block.filename,
        isEditing: false,
      });
    },
    [onOpenCanvas],
  );

  // 处理重试消息
  const handleRetry = useCallback(
    async (messageId: string) => {
      setRetryingMessageId(messageId);
      try {
        await retryMessage(messageId);
      } finally {
        setRetryingMessageId(null);
      }
    },
    [retryMessage],
  );

  // Provider 加载中时显示加载状态
  if (isProviderLoading) {
    return (
      <div className="flex flex-col h-full bg-surface">
        <div className="flex flex-col items-center justify-center h-full">
          <Loader2 className="w-8 h-8 text-accent animate-spin mb-4" />
          <p className="text-sm text-ink-500">正在加载 Provider...</p>
        </div>
      </div>
    );
  }

  // 无 Provider 时显示配置提示
  if (!hasAvailableProvider) {
    return (
      <div className="flex flex-col h-full bg-surface">
        <NoProviderPrompt
          onNavigateToConfig={
            onNavigate ? handleNavigateToProviderConfig : undefined
          }
        />
      </div>
    );
  }

  // 无会话时显示空状态
  if (!sessionId) {
    return (
      <div className="flex flex-col h-full bg-surface">
        <EmptyState />
      </div>
    );
  }

  return (
    <div className="flex h-full bg-surface">
      {/* 主聊天区域 */}
      <div className="flex flex-col flex-1">
        {/* 消息列表区域 */}
        <div className="flex-1 overflow-hidden px-4 py-4">
          {currentMessages.length === 0 ? (
            <EmptyState />
          ) : (
            <MessageList
              messages={currentMessages}
              isStreaming={isStreaming}
              partialContent={partialContent}
              onCopy={handleCopy}
              onOpenInCanvas={handleOpenInCanvas}
              onRetry={handleRetry}
              retryingMessageId={retryingMessageId}
              hasMoreMessages={hasMoreMessages}
              isLoadingMore={isLoadingMore}
              onLoadMore={handleLoadMore}
            />
          )}
        </div>

        {/* 输入区域 */}
        <div className="border-t border-ink-100 px-4 py-3 space-y-2">
          {/* 模型选择器 */}
          <div className="flex items-center">
            <CompactModelSelector disabled={isStreaming} />
          </div>

          {/* 输入栏 */}
          <Inputbar
            input={input}
            setInput={setInput}
            onSend={handleSend}
            onStop={handleStop}
            isLoading={isStreaming}
            disabled={!sessionId}
          />
        </div>
      </div>

      {/* 工作流状态面板 */}
      <WorkflowStatusPanel
        sessionId={sessionId || ""}
        isWorkflowActive={workflowEnabled}
        isWorkflowInitialized={isWorkflowInitialized}
        messageCount={messageCount}
        visualOperationCount={visualOperationCount}
        onInitializeWorkflow={handleInitializeWorkflow}
        onFinalizeWorkflow={handleFinalizeWorkflow}
      />
    </div>
  );
};

export default ChatPanel;
