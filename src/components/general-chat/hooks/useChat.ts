/**
 * @file useChat.ts
 * @description 聊天逻辑 Hook
 * @module components/general-chat/hooks/useChat
 *
 * 封装消息发送、Provider 配置等逻辑
 *
 * @requirements 2.1, 5.1, 5.2
 */

import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useGeneralChatStore } from "../store/useGeneralChatStore";
import type { Message, ProviderConfig } from "../types";

/**
 * 发送消息请求参数
 */
interface SendMessageRequest {
  session_id: string;
  content: string;
  event_name: string;
  provider?: string;
  model?: string;
}

/**
 * useChat Hook 配置
 */
interface UseChatOptions {
  /** 会话 ID */
  sessionId: string | null;
  /** Provider 配置 */
  providerConfig?: ProviderConfig;
  /** 消息发送成功回调 */
  onMessageSent?: (message: Message) => void;
  /** 消息发送失败回调 */
  onError?: (error: string) => void;
}

/**
 * 聊天逻辑 Hook
 */
export const useChat = (options: UseChatOptions) => {
  const { sessionId, providerConfig, onMessageSent, onError } = options;

  const { startStreaming } = useGeneralChatStore();

  /**
   * 发送消息
   */
  const sendMessage = useCallback(
    async (content: string) => {
      if (!sessionId || !content.trim()) {
        return;
      }

      try {
        // 构建事件名称
        const eventName = `general-chat-stream-${sessionId}`;

        // 调用 Tauri 命令发送消息
        const request: SendMessageRequest = {
          session_id: sessionId,
          content: content.trim(),
          event_name: eventName,
          provider: providerConfig?.providerName,
          model: providerConfig?.modelName,
        };

        const messageId = await invoke<string>("general_chat_send_message", {
          request,
        });

        startStreaming(messageId);

        // 消息发送成功
        if (onMessageSent) {
          const message: Message = {
            id: messageId,
            sessionId,
            role: "assistant",
            content: "",
            blocks: [],
            status: "streaming",
            createdAt: Date.now(),
          };
          onMessageSent(message);
        }
      } catch (error) {
        // 停止流式状态
        const { stopGeneration } = useGeneralChatStore.getState();
        stopGeneration();
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        onError?.(errorMessage);
      }
    },
    [sessionId, providerConfig, startStreaming, onMessageSent, onError],
  );

  /**
   * 停止生成
   */
  const stopGeneration = useCallback(async () => {
    if (!sessionId) return;

    try {
      await invoke("general_chat_stop_generation", {
        sessionId,
      });
      const { stopGeneration: stopGen } = useGeneralChatStore.getState();
      stopGen();
    } catch (error) {
      console.error("停止生成失败:", error);
    }
  }, [sessionId]);

  /**
   * 重新生成消息
   */
  const regenerateMessage = useCallback(async (messageId: string) => {
    // TODO: 实现重新生成逻辑
    // 1. 获取该消息之前的用户消息
    // 2. 删除该消息
    // 3. 重新发送用户消息
    console.log("重新生成消息:", messageId);
  }, []);

  return {
    sendMessage,
    stopGeneration,
    regenerateMessage,
  };
};

export default useChat;
