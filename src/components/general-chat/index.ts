/**
 * @file index.tsx
 * @description 通用对话功能主入口导出
 * @module components/general-chat
 * @requires ./GeneralChatPage
 * @exports GeneralChatPage - 通用对话页面组件
 */

// 主页面组件导出
export { default as GeneralChatPage } from "./GeneralChatPage";

// 类型导出（不触发 react-refresh 警告）
export type {
  Session,
  Message,
  MessageRole,
  MessageStatus,
  ContentBlock,
  ContentBlockType,
  CanvasState,
  CanvasContentType,
  UIState,
  StreamingState,
  ProviderConfig,
  MessageMetadata,
  GeneralChatPageProps,
  ChatPanelProps,
  MessageItemProps,
  InputBarProps,
  CanvasPanelProps,
} from "./types";

// 子模块组件导出
export {
  ChatPanel,
  MessageList,
  MessageItem,
  UserMessage,
  AssistantMessage,
  CodeBlock,
  ErrorBoundary,
} from "./chat";
export { CanvasPanel, CodePreview, MarkdownPreview } from "./canvas";
export { useGeneralChatStore } from "./store";
export { useStreaming, useChat, useSession } from "./hooks";
