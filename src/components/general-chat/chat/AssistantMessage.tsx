/**
 * @file AssistantMessage.tsx
 * @description AI 助手消息组件 - 显示 AI 响应，支持 Markdown 渲染和错误状态
 * @module components/general-chat/chat/AssistantMessage
 *
 * @requirements 6.2, 6.7, 2.6, 9.2, 9.3, 9.5
 */

import React, { useState, useMemo } from "react";
import type { Message, ContentBlock } from "../types";
import { CodeBlock } from "./CodeBlock";
import { ErrorDisplay } from "./ErrorDisplay";
import { ImageMessage } from "./ImageMessage";

interface AssistantMessageProps {
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
  /** 重新生成回调 */
  onRegenerate?: () => void;
  /** 重试回调（错误状态时使用） */
  onRetry?: () => void;
  /** 是否正在重试 */
  isRetrying?: boolean;
}

/**
 * 解析 Markdown 内容，提取代码块和图片
 */
const parseContent = (content: string): ContentBlock[] => {
  const blocks: ContentBlock[] = [];
  const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g;
  let lastIndex = 0;
  let match;

  while ((match = codeBlockRegex.exec(content)) !== null) {
    // 添加代码块之前的文本
    if (match.index > lastIndex) {
      const text = content.slice(lastIndex, match.index).trim();
      if (text) {
        blocks.push({ type: "text", content: text });
      }
    }

    // 添加代码块
    blocks.push({
      type: "code",
      content: match[2].trim(),
      language: match[1] || "plaintext",
    });

    lastIndex = match.index + match[0].length;
  }

  // 添加剩余的文本
  if (lastIndex < content.length) {
    const text = content.slice(lastIndex).trim();
    if (text) {
      blocks.push({ type: "text", content: text });
    }
  }

  // 如果没有解析出任何块，返回整个内容作为文本
  if (blocks.length === 0) {
    blocks.push({ type: "text", content });
  }

  return blocks;
};

/**
 * AI 助手消息组件
 */
export const AssistantMessage: React.FC<AssistantMessageProps> = ({
  message,
  isStreaming,
  streamingContent,
  onCopy,
  onOpenInCanvas,
  onRegenerate,
  onRetry,
  isRetrying = false,
}) => {
  const [showActions, setShowActions] = useState(false);
  const [copied, setCopied] = useState(false);

  // 判断是否为错误状态
  const isError = message.status === "error" && message.error;

  // 使用流式内容或消息内容
  const displayContent =
    isStreaming && streamingContent ? streamingContent : message.content;

  // 解析内容块（仅用于文本内容的 Markdown 解析）
  const parsedContentBlocks = useMemo(
    () => parseContent(displayContent),
    [displayContent],
  );

  // 合并消息中的图片块和解析出的内容块
  const allContentBlocks = useMemo(() => {
    const imageBlocks = message.blocks.filter(
      (block) => block.type === "image",
    );
    return [...imageBlocks, ...parsedContentBlocks];
  }, [message.blocks, parsedContentBlocks]);

  const handleCopy = () => {
    onCopy(message.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      className="flex justify-start"
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => setShowActions(false)}
    >
      <div className="max-w-[85%] flex flex-col gap-1">
        {/* 头像和标签 */}
        <div className="flex items-center gap-2 mb-1">
          <div
            className={`w-6 h-6 rounded-full flex items-center justify-center ${isError ? "bg-red-100" : "bg-accent/10"}`}
          >
            <svg
              className={`w-4 h-4 ${isError ? "text-red-500" : "text-accent"}`}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
              />
            </svg>
          </div>
          <span className="text-xs text-ink-500">AI 助手</span>
          {isStreaming && (
            <span className="flex items-center gap-1 text-xs text-accent">
              <span className="w-1.5 h-1.5 bg-accent rounded-full animate-pulse" />
              生成中...
            </span>
          )}
          {isError && (
            <span className="flex items-center gap-1 text-xs text-red-500">
              <span className="w-1.5 h-1.5 bg-red-500 rounded-full" />
              发送失败
            </span>
          )}
        </div>

        {/* 错误状态显示 */}
        {isError && message.error && (
          <ErrorDisplay
            error={message.error}
            onRetry={onRetry}
            isRetrying={isRetrying}
          />
        )}

        {/* 消息内容（非错误状态或有部分内容时显示） */}
        {(!isError ||
          displayContent ||
          message.blocks.some((block) => block.type === "image")) && (
          <div
            className={`bg-surface-secondary rounded-2xl rounded-tl-sm px-4 py-3 ${isError ? "opacity-60" : ""}`}
          >
            <div className="prose prose-sm max-w-none">
              {allContentBlocks.map((block, index) => (
                <div key={index} className="mb-2 last:mb-0">
                  {block.type === "code" ? (
                    <CodeBlock
                      code={block.content}
                      language={block.language || "plaintext"}
                      onCopy={() => onCopy(block.content)}
                      onOpenInCanvas={() => onOpenInCanvas(block)}
                    />
                  ) : block.type === "image" ? (
                    <ImageMessage block={block} />
                  ) : (
                    <p className="text-sm text-ink-800 whitespace-pre-wrap break-words">
                      {block.content}
                    </p>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* 操作按钮 */}
        {showActions && !isStreaming && !isError && (
          <div className="flex items-center gap-1 px-1 mt-1">
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
            {onRegenerate && (
              <button
                onClick={onRegenerate}
                className="p-1 text-ink-400 hover:text-ink-600 transition-colors"
                title="重新生成"
              >
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
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                  />
                </svg>
              </button>
            )}
          </div>
        )}

        {/* 元数据 */}
        {message.metadata && !isStreaming && !isError && (
          <div className="flex items-center gap-2 px-1 text-xs text-ink-400">
            {message.metadata.model && <span>{message.metadata.model}</span>}
            {message.metadata.tokens && (
              <span>· {message.metadata.tokens} tokens</span>
            )}
            {message.metadata.duration && (
              <span>· {(message.metadata.duration / 1000).toFixed(1)}s</span>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default AssistantMessage;
