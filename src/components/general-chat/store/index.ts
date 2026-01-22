/**
 * @file index.ts
 * @description 状态管理模块导出
 * @module components/general-chat/store
 *
 * 导出 Zustand Store 和相关选择器 Hooks
 */

export {
  useGeneralChatStore,
  useCurrentSession,
  useCurrentMessages,
  useStreamingState,
  useUIState,
  useCanvasState,
  useSessions,
  useIsStreaming,
} from "./useGeneralChatStore";

export type { GeneralChatState } from "./useGeneralChatStore";
