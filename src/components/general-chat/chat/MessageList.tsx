/**
 * @file MessageList.tsx
 * @description 消息列表组件 - 使用虚拟滚动渲染对话消息，支持分页加载
 * @module components/general-chat/chat/MessageList
 *
 * @requirements 10.1, 10.2, 2.6, 9.5
 *
 * 虚拟滚动实现说明：
 * - 使用 @tanstack/react-virtual 实现虚拟滚动
 * - 支持动态高度消息（聊天消息高度不固定）
 * - 保持自动滚动到底部功能
 * - 流式消息更新时滚动行为正常
 *
 * 分页加载实现说明：
 * - 滚动到顶部时自动加载更多历史消息
 * - 每次加载 20-50 条消息（可配置）
 * - 与虚拟滚动兼容
 */

import React, { useRef, useEffect, useCallback } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";

import type { Message, ContentBlock } from "../types";
import { MessageItem } from "./MessageItem";

/** 虚拟滚动启用阈值 - 超过此数量启用虚拟滚动 */
const VIRTUAL_SCROLL_THRESHOLD = 50;

/** 预估消息高度（用于初始渲染） */
const ESTIMATED_MESSAGE_HEIGHT = 120;

/** 过扫描数量（在可视区域外额外渲染的消息数） */
const OVERSCAN_COUNT = 5;

/** 触发加载更多的滚动阈值（距离顶部的像素数） */
const LOAD_MORE_THRESHOLD = 100;

interface MessageListProps {
  /** 消息列表 */
  messages: Message[];
  /** 是否正在流式生成 */
  isStreaming: boolean;
  /** 流式生成的部分内容 */
  partialContent: string;
  /** 复制内容回调 */
  onCopy: (content: string) => void;
  /** 在画布中打开回调 */
  onOpenInCanvas: (block: ContentBlock) => void;
  /** 重新生成回调 */
  onRegenerate?: (messageId: string) => void;
  /** 重试回调 */
  onRetry?: (messageId: string) => void;
  /** 正在重试的消息 ID */
  retryingMessageId?: string | null;
  /** 是否还有更多消息可加载 */
  hasMoreMessages?: boolean;
  /** 是否正在加载更多消息 */
  isLoadingMore?: boolean;
  /** 加载更多消息回调 */
  onLoadMore?: () => void;
}

/**
 * 虚拟滚动消息列表组件
 *
 * 当消息数量超过阈值时启用虚拟滚动，
 * 确保大量消息时 DOM 元素数量受控。
 * 支持滚动到顶部时加载更多历史消息。
 *
 * @requirements 10.1, 10.2
 */
export const MessageList: React.FC<MessageListProps> = ({
  messages,
  isStreaming,
  partialContent,
  onCopy,
  onOpenInCanvas,
  onRegenerate,
  onRetry,
  retryingMessageId,
  hasMoreMessages = true,
  isLoadingMore = false,
  onLoadMore,
}) => {
  const parentRef = useRef<HTMLDivElement>(null);
  const isAtBottomRef = useRef(true);
  const prevMessageCountRef = useRef(messages.length);
  const prevFirstMessageIdRef = useRef<string | null>(null);

  // 判断是否需要启用虚拟滚动
  const useVirtualScroll = messages.length > VIRTUAL_SCROLL_THRESHOLD;

  // 创建虚拟化器
  const virtualizer = useVirtualizer({
    count: messages.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ESTIMATED_MESSAGE_HEIGHT,
    overscan: OVERSCAN_COUNT,
    // 启用动态高度测量
    measureElement: (element) => {
      return element.getBoundingClientRect().height;
    },
  });

  // 检查是否在底部
  const checkIsAtBottom = useCallback(() => {
    const parent = parentRef.current;
    if (!parent) return true;

    const { scrollTop, scrollHeight, clientHeight } = parent;
    // 允许 50px 的误差
    return scrollHeight - scrollTop - clientHeight < 50;
  }, []);

  // 滚动到底部
  const scrollToBottom = useCallback(
    (behavior: "smooth" | "auto" = "smooth") => {
      if (useVirtualScroll) {
        virtualizer.scrollToIndex(messages.length - 1, {
          align: "end",
          // 使用类型断言解决 @tanstack/react-virtual 与 DOM ScrollBehavior 类型不兼容问题
          behavior: behavior as "smooth" | "auto",
        });
      } else {
        const parent = parentRef.current;
        if (parent) {
          parent.scrollTo({
            top: parent.scrollHeight,
            behavior,
          });
        }
      }
    },
    [useVirtualScroll, virtualizer, messages.length],
  );

  // 监听滚动事件，更新是否在底部的状态
  useEffect(() => {
    const parent = parentRef.current;
    if (!parent) return;

    const handleScroll = () => {
      isAtBottomRef.current = checkIsAtBottom();

      // 检查是否滚动到顶部，触发加载更多
      if (
        onLoadMore &&
        hasMoreMessages &&
        !isLoadingMore &&
        parent.scrollTop < LOAD_MORE_THRESHOLD
      ) {
        onLoadMore();
      }
    };

    parent.addEventListener("scroll", handleScroll, { passive: true });
    return () => parent.removeEventListener("scroll", handleScroll);
  }, [checkIsAtBottom, onLoadMore, hasMoreMessages, isLoadingMore]);

  // 新消息到达时自动滚动到底部
  useEffect(() => {
    const messageCountChanged = messages.length !== prevMessageCountRef.current;
    const firstMessageId = messages.length > 0 ? messages[0].id : null;
    const firstMessageChanged =
      firstMessageId !== prevFirstMessageIdRef.current;

    // 更新引用
    const prevCount = prevMessageCountRef.current;
    prevMessageCountRef.current = messages.length;
    prevFirstMessageIdRef.current = firstMessageId;

    // 如果是加载更多历史消息（第一条消息 ID 变化），保持滚动位置
    if (firstMessageChanged && messages.length > prevCount) {
      // 加载了更多历史消息，需要调整滚动位置以保持当前视图
      const parent = parentRef.current;
      if (parent && useVirtualScroll) {
        // 虚拟滚动模式下，滚动到新加载消息数量的位置
        const newMessagesCount = messages.length - prevCount;
        requestAnimationFrame(() => {
          virtualizer.scrollToIndex(newMessagesCount, {
            align: "start",
            behavior: "auto" as "smooth" | "auto",
          });
        });
      }
      return;
    }

    // 如果有新消息且用户在底部，自动滚动
    if (messageCountChanged && isAtBottomRef.current) {
      // 使用 requestAnimationFrame 确保 DOM 更新后再滚动
      requestAnimationFrame(() => {
        scrollToBottom("smooth");
      });
    }
  }, [
    messages.length,
    messages,
    scrollToBottom,
    useVirtualScroll,
    virtualizer,
  ]);

  // 流式内容更新时保持滚动到底部
  useEffect(() => {
    if (isStreaming && isAtBottomRef.current) {
      requestAnimationFrame(() => {
        scrollToBottom("auto");
      });
    }
  }, [isStreaming, partialContent, scrollToBottom]);

  // 渲染单个消息项
  const renderMessageItem = useCallback(
    (message: Message, index: number) => {
      const isLast = index === messages.length - 1;
      const isStreamingMessage =
        isLast && isStreaming && message.role === "assistant";
      const isRetrying = retryingMessageId === message.id;

      return (
        <MessageItem
          key={message.id}
          message={message}
          isStreaming={isStreamingMessage}
          streamingContent={isStreamingMessage ? partialContent : undefined}
          onCopy={onCopy}
          onOpenInCanvas={onOpenInCanvas}
          onRegenerate={
            onRegenerate ? () => onRegenerate(message.id) : undefined
          }
          onRetry={onRetry ? () => onRetry(message.id) : undefined}
          isRetrying={isRetrying}
        />
      );
    },
    [
      messages.length,
      isStreaming,
      partialContent,
      onCopy,
      onOpenInCanvas,
      onRegenerate,
      onRetry,
      retryingMessageId,
    ],
  );

  // 非虚拟滚动模式（消息数量较少时）
  if (!useVirtualScroll) {
    return (
      <div
        ref={parentRef}
        className="flex flex-col gap-4 h-full overflow-y-auto"
        data-testid="message-list"
      >
        {/* 加载更多指示器 */}
        {isLoadingMore && (
          <div className="flex justify-center py-4">
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <div className="w-4 h-4 border-2 border-primary border-t-transparent rounded-full animate-spin" />
              <span>加载更多消息...</span>
            </div>
          </div>
        )}
        {/* 没有更多消息提示 */}
        {!hasMoreMessages && messages.length > 0 && (
          <div className="flex justify-center py-2">
            <span className="text-xs text-muted-foreground">
              已加载全部消息
            </span>
          </div>
        )}
        {messages.map((message, index) => renderMessageItem(message, index))}
      </div>
    );
  }

  // 虚拟滚动模式
  const virtualItems = virtualizer.getVirtualItems();

  return (
    <div
      ref={parentRef}
      className="h-full overflow-y-auto"
      data-testid="message-list-virtual"
    >
      {/* 虚拟滚动容器 */}
      <div
        className="relative w-full"
        style={{
          height: `${virtualizer.getTotalSize()}px`,
        }}
      >
        {/* 加载更多指示器（固定在顶部） */}
        {isLoadingMore && (
          <div className="absolute top-0 left-0 right-0 flex justify-center py-4 bg-background/80 backdrop-blur-sm z-10">
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <div className="w-4 h-4 border-2 border-primary border-t-transparent rounded-full animate-spin" />
              <span>加载更多消息...</span>
            </div>
          </div>
        )}
        {/* 没有更多消息提示 */}
        {!hasMoreMessages && messages.length > 0 && !isLoadingMore && (
          <div className="absolute top-0 left-0 right-0 flex justify-center py-2 z-10">
            <span className="text-xs text-muted-foreground">
              已加载全部消息
            </span>
          </div>
        )}
        {/* 虚拟滚动内容 */}
        <div
          className="absolute top-0 left-0 w-full"
          style={{
            transform: `translateY(${virtualItems[0]?.start ?? 0}px)`,
          }}
        >
          {virtualItems.map((virtualItem) => {
            const message = messages[virtualItem.index];
            return (
              <div
                key={virtualItem.key}
                data-index={virtualItem.index}
                ref={virtualizer.measureElement}
                className="pb-4"
              >
                {renderMessageItem(message, virtualItem.index)}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};

export default MessageList;
