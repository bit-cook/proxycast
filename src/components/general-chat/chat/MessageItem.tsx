/**
 * @file MessageItem.tsx
 * @description 消息项组件 - 根据角色渲染不同样式的消息
 * @module components/general-chat/chat/MessageItem
 *
 * @requirements 6.1, 6.2, 6.7, 2.6, 9.5
 */

import React from "react";

import type { Message, ContentBlock } from "../types";
import { UserMessage } from "./UserMessage";
import { AssistantMessage } from "./AssistantMessage";

interface MessageItemProps {
  /** 消息数据 */
  message: Message;
  /** 是否正在流式生成 */
  isStreaming: boolean;
  /** 流式生成的部分内容 */
  streamingContent?: string;
  /** 复制内容回调 */
  onCopy: (content: string) => void;
  /** 在画布中打开回调 */
  onOpenInCanvas: (block: ContentBlock) => void;
  /** 重新生成回调 (仅 AI 消息) */
  onRegenerate?: () => void;
  /** 重试回调（错误状态时使用） */
  onRetry?: () => void;
  /** 是否正在重试 */
  isRetrying?: boolean;
}

/**
 * 消息项组件
 */
export const MessageItem: React.FC<MessageItemProps> = ({
  message,
  isStreaming,
  streamingContent,
  onCopy,
  onOpenInCanvas,
  onRegenerate,
  onRetry,
  isRetrying,
}) => {
  if (message.role === "user") {
    return <UserMessage message={message} onCopy={onCopy} />;
  }

  if (message.role === "assistant") {
    return (
      <AssistantMessage
        message={message}
        isStreaming={isStreaming}
        streamingContent={streamingContent}
        onCopy={onCopy}
        onOpenInCanvas={onOpenInCanvas}
        onRegenerate={onRegenerate}
        onRetry={onRetry}
        isRetrying={isRetrying}
      />
    );
  }

  // 系统消息 (暂不显示)
  return null;
};

export default MessageItem;
