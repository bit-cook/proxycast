/**
 * @file useStreaming.ts
 * @description 流式响应处理 Hook
 * @module components/general-chat/hooks/useStreaming
 *
 * 处理 Tauri 事件监听和流式内容累积
 *
 * @requirements 2.2, 2.5
 */

import { useEffect, useCallback, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useGeneralChatStore } from "../store/useGeneralChatStore";

/**
 * 流式事件类型
 */
interface StreamEvent {
  type: "start" | "delta" | "done" | "error";
  message_id?: string;
  content?: string;
  message?: string;
}

/**
 * useStreaming Hook 配置
 */
interface UseStreamingOptions {
  /** 会话 ID */
  sessionId: string | null;
  /** 事件名称 */
  eventName?: string;
  /** 开始回调 */
  onStart?: (messageId: string) => void;
  /** 增量内容回调 */
  onDelta?: (content: string) => void;
  /** 完成回调 */
  onDone?: (messageId: string, content: string) => void;
  /** 错误回调 */
  onError?: (error: string) => void;
}

/**
 * 流式响应处理 Hook
 *
 * 监听 Tauri 事件，处理流式响应
 */
export const useStreaming = (options: UseStreamingOptions) => {
  const {
    sessionId,
    eventName = "general-chat-stream",
    onStart,
    onDelta,
    onDone,
    onError,
  } = options;

  const { startStreaming, appendStreamingContent } = useGeneralChatStore();

  const unlistenRef = useRef<UnlistenFn | null>(null);
  const contentRef = useRef<string>("");

  // 处理流式事件
  const handleStreamEvent = useCallback(
    (event: { payload: StreamEvent }) => {
      const { type, message_id, content, message } = event.payload;

      switch (type) {
        case "start":
          contentRef.current = "";
          startStreaming(message_id || "");
          onStart?.(message_id || "");
          break;

        case "delta":
          if (content) {
            contentRef.current += content;
            appendStreamingContent(content);
            onDelta?.(content);
          }
          break;

        case "done": {
          const { finalizeMessage } = useGeneralChatStore.getState();
          finalizeMessage();
          onDone?.(message_id || "", contentRef.current);
          contentRef.current = "";
          break;
        }

        case "error": {
          const { stopGeneration: stopGen } = useGeneralChatStore.getState();
          stopGen();
          onError?.(message || "未知错误");
          contentRef.current = "";
          break;
        }
      }
    },
    [startStreaming, appendStreamingContent, onStart, onDelta, onDone, onError],
  );

  // 设置事件监听
  useEffect(() => {
    if (!sessionId) return;

    const setupListener = async () => {
      // 清理之前的监听器
      if (unlistenRef.current) {
        unlistenRef.current();
      }

      // 设置新的监听器
      const eventKey = `${eventName}-${sessionId}`;
      unlistenRef.current = await listen<StreamEvent>(
        eventKey,
        handleStreamEvent,
      );
    };

    setupListener();

    // 清理函数
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, [sessionId, eventName, handleStreamEvent]);

  // 停止生成
  const stopGeneration = useCallback(() => {
    const { stopGeneration: stopGen } = useGeneralChatStore.getState();
    stopGen();
    contentRef.current = "";
  }, []);

  return {
    stopGeneration,
  };
};

export default useStreaming;
