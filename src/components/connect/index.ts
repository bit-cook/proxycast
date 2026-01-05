/**
 * @file ProxyCast Connect 组件入口
 * @description 导出 Connect 功能相关的所有组件
 * @module components/connect
 */

// 组件导出
export { ConnectConfirmDialog } from "./ConnectConfirmDialog";
export type { ConnectConfirmDialogProps } from "./ConnectConfirmDialog";

// 错误提示组件导出
// _Requirements: 7.1, 7.2, 7.3, 7.4_
export { ConnectErrorAlert } from "./ConnectErrorToast";
export type { ConnectErrorAlertProps } from "./ConnectErrorToast";

// 错误提示工具函数从 lib/utils/connectError 导出
// 使用方式: import { showDeepLinkError } from "@/lib/utils/connectError";
