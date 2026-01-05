/**
 * @file Connect 错误处理工具函数
 * @description 提供 ProxyCast Connect 功能的错误提示 Toast 通知工具函数
 * @module lib/utils/connectError
 *
 * _Requirements: 7.1, 7.2, 7.3, 7.4_
 */

import { toast } from "sonner";

/**
 * Connect 错误类型
 */
export type ConnectErrorType =
  | "deep_link_parse"
  | "registry_load"
  | "registry_no_cache"
  | "api_key_save";

/**
 * Connect 错误信息
 */
export interface ConnectErrorInfo {
  /** 错误类型 */
  type: ConnectErrorType;
  /** 错误消息 */
  message: string;
  /** 原始错误代码（可选） */
  code?: string;
}

/**
 * 错误配置
 */
interface ErrorConfig {
  title: string;
  description: (message: string) => string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

/**
 * 获取错误配置
 */
function getErrorConfig(
  type: ConnectErrorType,
  onRetry?: () => void,
): ErrorConfig {
  switch (type) {
    case "deep_link_parse":
      // _Requirements: 7.1_
      return {
        title: "链接解析失败",
        description: (msg) => msg || "Deep Link 格式无效，请检查链接是否正确",
      };

    case "registry_load":
      // _Requirements: 7.2_
      return {
        title: "注册表加载失败",
        description: (msg) =>
          msg || "无法从远程加载中转商注册表，已使用本地缓存",
      };

    case "registry_no_cache":
      // _Requirements: 7.3_
      return {
        title: "注册表不可用",
        description: (msg) =>
          msg || "无法加载中转商注册表且没有本地缓存，请检查网络连接",
        action: onRetry
          ? {
              label: "重试",
              onClick: onRetry,
            }
          : undefined,
      };

    case "api_key_save":
      // _Requirements: 7.4_
      return {
        title: "保存失败",
        description: (msg) => msg || "API Key 保存失败，请重试",
      };

    default:
      return {
        title: "操作失败",
        description: (msg) => msg || "发生未知错误",
      };
  }
}

/**
 * 显示 Connect 错误 Toast
 *
 * 根据错误类型显示对应的错误提示。
 *
 * ## 错误类型
 *
 * - `deep_link_parse`: Deep Link 解析错误（Requirements 7.1）
 * - `registry_load`: Registry 加载失败，已回退到缓存（Requirements 7.2）
 * - `registry_no_cache`: Registry 无缓存且加载失败（Requirements 7.3）
 * - `api_key_save`: API Key 存储失败（Requirements 7.4）
 *
 * @param error - 错误信息
 * @param onRetry - 重试回调（仅用于 registry_no_cache 类型）
 */
export function showConnectError(
  error: ConnectErrorInfo,
  onRetry?: () => void,
): void {
  const config = getErrorConfig(error.type, onRetry);

  // 使用 sonner 的 error toast
  toast.error(config.title, {
    description: config.description(error.message),
    duration: error.type === "registry_no_cache" ? 10000 : 5000,
    action: config.action
      ? {
          label: config.action.label,
          onClick: config.action.onClick,
        }
      : undefined,
  });
}

/**
 * 显示 Deep Link 解析错误
 * _Requirements: 7.1_
 *
 * @param message - 错误消息
 * @param code - 错误代码
 */
export function showDeepLinkError(message: string, code?: string): void {
  showConnectError({
    type: "deep_link_parse",
    message,
    code,
  });
}

/**
 * 显示 Registry 加载失败（已回退到缓存）
 * _Requirements: 7.2_
 *
 * @param message - 错误消息
 */
export function showRegistryLoadError(message?: string): void {
  showConnectError({
    type: "registry_load",
    message: message || "无法从远程加载中转商注册表，已使用本地缓存",
  });
}

/**
 * 显示 Registry 不可用错误（无缓存）
 * _Requirements: 7.3_
 *
 * @param message - 错误消息
 * @param onRetry - 重试回调
 */
export function showRegistryNoCacheError(
  message?: string,
  onRetry?: () => void,
): void {
  showConnectError(
    {
      type: "registry_no_cache",
      message: message || "无法加载中转商注册表且没有本地缓存",
    },
    onRetry,
  );
}

/**
 * 显示 API Key 保存失败错误
 * _Requirements: 7.4_
 *
 * @param message - 错误消息
 */
export function showApiKeySaveError(message?: string): void {
  showConnectError({
    type: "api_key_save",
    message: message || "API Key 保存失败，请重试",
  });
}

/**
 * 获取错误配置（供组件使用）
 */
export { getErrorConfig };
