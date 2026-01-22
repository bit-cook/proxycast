/**
 * @file GeneralChatPage.tsx
 * @description 通用对话主页面 - 三栏布局
 * @module components/general-chat/GeneralChatPage
 *
 * @requirements 3.1, 3.5, 9.4
 */

import React, { useState, useCallback, useEffect, useRef } from "react";
import { ChatPanel } from "./chat/ChatPanel";
import { CanvasPanel } from "./canvas/CanvasPanel";
import { ErrorBoundary } from "./chat/ErrorBoundary";
import { useGeneralChatStore } from "./store/useGeneralChatStore";
import type { CanvasState, GeneralChatPageProps } from "./types";
import { DEFAULT_CANVAS_STATE } from "./types";

/**
 * 通用对话主页面
 *
 * 三栏布局：
 * - 左侧：会话列表（复用 ChatSidebar）
 * - 中间：聊天区域
 * - 右侧：画布面板（可折叠）
 */
export const GeneralChatPage: React.FC<GeneralChatPageProps> = ({
  initialSessionId,
  onNavigate,
}) => {
  const { currentSessionId, selectSession, sessions, createSession } =
    useGeneralChatStore();

  // 画布状态
  const [canvasState, setCanvasState] =
    useState<CanvasState>(DEFAULT_CANVAS_STATE);

  // 使用 ref 防止重复创建会话
  const sessionCreatedRef = useRef(false);

  // 初始化：如果有初始会话 ID，选择它；否则如果没有会话，创建一个
  useEffect(() => {
    if (initialSessionId) {
      selectSession(initialSessionId);
    } else if (
      sessions.length === 0 &&
      !currentSessionId &&
      !sessionCreatedRef.current
    ) {
      // 如果没有会话，创建一个新会话（只创建一次）
      sessionCreatedRef.current = true;
      createSession();
    }
  }, [
    initialSessionId,
    selectSession,
    sessions.length,
    currentSessionId,
    createSession,
  ]);

  // 打开画布
  const handleOpenCanvas = useCallback((state: CanvasState) => {
    setCanvasState(state);
  }, []);

  // 关闭画布
  const handleCloseCanvas = useCallback(() => {
    setCanvasState(DEFAULT_CANVAS_STATE);
  }, []);

  // 画布内容变更
  const handleCanvasContentChange = useCallback((content: string) => {
    setCanvasState((prev) => ({ ...prev, content }));
  }, []);

  return (
    <div className="flex h-full bg-background">
      {/* 中间：聊天区域 - 使用 ErrorBoundary 包裹 */}
      <div className="flex-1 flex flex-col min-w-0">
        <ErrorBoundary
          componentName="ChatPanel"
          onError={(error, errorInfo) => {
            console.error(
              "[GeneralChatPage] ChatPanel 渲染错误:",
              error.message,
            );
            console.error(
              "[GeneralChatPage] 组件堆栈:",
              errorInfo.componentStack,
            );
          }}
        >
          <ChatPanel
            sessionId={currentSessionId}
            onOpenCanvas={handleOpenCanvas}
            onNavigate={onNavigate}
          />
        </ErrorBoundary>
      </div>

      {/* 右侧：画布面板 - 使用 ErrorBoundary 包裹 */}
      {canvasState.isOpen && (
        <div className="w-[400px] flex-shrink-0 border-l border-ink-200">
          <ErrorBoundary
            componentName="CanvasPanel"
            onError={(error, errorInfo) => {
              console.error(
                "[GeneralChatPage] CanvasPanel 渲染错误:",
                error.message,
              );
              console.error(
                "[GeneralChatPage] 组件堆栈:",
                errorInfo.componentStack,
              );
            }}
          >
            <CanvasPanel
              state={canvasState}
              onClose={handleCloseCanvas}
              onContentChange={handleCanvasContentChange}
            />
          </ErrorBoundary>
        </div>
      )}
    </div>
  );
};

export default GeneralChatPage;
