/**
 * @file useStreaming Hook
 * @description 流式响应处理 Hook
 * @module components/chat/hooks/useStreaming
 */

import { useCallback } from "react";
import { safeInvoke, safeListen } from "@/lib/dev-bridge";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { Message } from "../types";

/**
 * 流式响应事件数据
 */
interface StreamChunkEvent {
  content: string;
  done: boolean;
}

/**
 * 流式对话选项
 */
interface StreamChatOptions {
  /** 项目 ID（可选，用于注入项目上下文） */
  projectId?: string;
}

/**
 * 流式响应处理 Hook
 *
 * 提供与后端的流式通信能力
 *
 * @returns 流式对话方法
 */
export function useStreaming() {
  /**
   * 流式对话
   *
   * @param messages - 消息历史
   * @param onChunk - 收到数据块时的回调
   * @param signal - AbortSignal 用于取消请求
   * @param options - 可选配置（包含 projectId）
   */
  const streamChat = useCallback(
    async (
      messages: Message[],
      onChunk: (chunk: string) => void,
      signal?: AbortSignal,
      options?: StreamChatOptions,
    ): Promise<void> => {
      let unlisten: UnlistenFn | null = null;
      let isAborted = false;

      // 监听中断信号
      if (signal) {
        signal.addEventListener("abort", () => {
          isAborted = true;
          unlisten?.();
        });
      }

      try {
        // 监听流式响应事件
        unlisten = await safeListen<StreamChunkEvent>(
          "chat-stream-chunk",
          (event) => {
            if (isAborted) return;
            if (event.payload.content) {
              onChunk(event.payload.content);
            }
          },
        );

        // 调用 Tauri 命令开始流式对话
        // 注意：这里使用现有的 agent_chat_stream 命令
        // 如果需要独立的通用对话命令，可以后续添加
        await safeInvoke("agent_chat_stream", {
          messages: messages.map((m) => ({
            role: m.role,
            content: m.content,
          })),
          projectId: options?.projectId,
        });
      } catch (err) {
        if (!isAborted) {
          throw err;
        }
      } finally {
        unlisten?.();
      }
    },
    [],
  );

  return { streamChat };
}
