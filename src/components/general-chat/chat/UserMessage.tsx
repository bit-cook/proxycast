/**
 * @file UserMessage.tsx
 * @description 用户消息组件 - 显示用户发送的消息
 * @module components/general-chat/chat/UserMessage
 *
 * @requirements 6.1
 */

import React, { useState } from "react";
import type { Message } from "../types";
import { ImageMessage } from "./ImageMessage";

interface UserMessageProps {
  /** 消息数据 */
  message: Message;
  /** 复制内容回调 */
  onCopy: (content: string) => void;
}

/**
 * 用户消息组件
 */
export const UserMessage: React.FC<UserMessageProps> = ({
  message,
  onCopy,
}) => {
  const [showActions, setShowActions] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    onCopy(message.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // 分离文本和图片内容
  const textBlocks = message.blocks.filter((block) => block.type === "text");
  const imageBlocks = message.blocks.filter((block) => block.type === "image");
  const hasText =
    textBlocks.length > 0 && textBlocks.some((block) => block.content.trim());
  const hasImages = imageBlocks.length > 0;

  return (
    <div
      className="flex justify-end"
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => setShowActions(false)}
    >
      <div className="max-w-[80%] flex flex-col items-end gap-2">
        {/* 图片内容 */}
        {hasImages && (
          <div className="flex flex-col gap-2 items-end">
            {imageBlocks.map((block, index) => (
              <ImageMessage key={index} block={block} />
            ))}
          </div>
        )}

        {/* 文本消息气泡 */}
        {hasText && (
          <div className="bg-accent text-white rounded-2xl rounded-tr-sm px-4 py-2.5">
            <p className="text-sm whitespace-pre-wrap break-words">
              {message.content}
            </p>
          </div>
        )}

        {/* 操作按钮 */}
        {showActions && (
          <div className="flex items-center gap-1 px-1">
            <button
              onClick={handleCopy}
              className="p-1 text-ink-400 hover:text-ink-600 transition-colors"
              title={copied ? "已复制" : "复制"}
            >
              {copied ? (
                <svg
                  className="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              ) : (
                <svg
                  className="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                  />
                </svg>
              )}
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default UserMessage;
