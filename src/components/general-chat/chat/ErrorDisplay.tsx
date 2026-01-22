/**
 * @file ErrorDisplay.tsx
 * @description 错误显示组件 - 显示消息错误状态和重试按钮
 * @module components/general-chat/chat/ErrorDisplay
 *
 * 根据不同的错误类型显示相应的错误信息和操作按钮
 *
 * @requirements 2.6, 9.2, 9.3, 9.5
 */

import React, { useState, useEffect } from "react";
import {
  AlertCircle,
  RefreshCw,
  WifiOff,
  Clock,
  AlertTriangle,
  XCircle,
} from "lucide-react";
import type { ErrorInfo, ErrorCode } from "../types";

interface ErrorDisplayProps {
  /** 错误信息 */
  error: ErrorInfo;
  /** 重试回调 */
  onRetry?: () => void;
  /** 是否正在重试 */
  isRetrying?: boolean;
}

/**
 * 根据错误代码获取对应的图标
 */
const getErrorIcon = (code: ErrorCode): React.ReactNode => {
  const iconClass = "w-5 h-5";

  switch (code) {
    case "NETWORK_ERROR":
      return <WifiOff className={iconClass} />;
    case "TIMEOUT":
      return <Clock className={iconClass} />;
    case "RATE_LIMIT":
      return <AlertTriangle className={iconClass} />;
    case "TOKEN_LIMIT":
      return <XCircle className={iconClass} />;
    case "AUTH_ERROR":
      return <AlertCircle className={iconClass} />;
    case "SERVER_ERROR":
    case "PROVIDER_ERROR":
    case "UNKNOWN_ERROR":
    default:
      return <AlertCircle className={iconClass} />;
  }
};

/**
 * 根据错误代码获取背景颜色类
 */
const getErrorColorClass = (code: ErrorCode): string => {
  switch (code) {
    case "NETWORK_ERROR":
    case "TIMEOUT":
      return "bg-amber-50 border-amber-200 text-amber-800";
    case "RATE_LIMIT":
      return "bg-orange-50 border-orange-200 text-orange-800";
    case "TOKEN_LIMIT":
    case "AUTH_ERROR":
      return "bg-red-50 border-red-200 text-red-800";
    case "SERVER_ERROR":
    case "PROVIDER_ERROR":
    case "UNKNOWN_ERROR":
    default:
      return "bg-red-50 border-red-200 text-red-700";
  }
};

/**
 * 错误显示组件
 *
 * 显示错误信息、图标和重试按钮（如果错误可重试）
 */
export const ErrorDisplay: React.FC<ErrorDisplayProps> = ({
  error,
  onRetry,
  isRetrying = false,
}) => {
  const [countdown, setCountdown] = useState<number | null>(null);

  // 处理 rate limit 倒计时
  useEffect(() => {
    if (
      error.code === "RATE_LIMIT" &&
      error.retryAfter &&
      error.retryAfter > 0
    ) {
      setCountdown(error.retryAfter);

      const timer = setInterval(() => {
        setCountdown((prev) => {
          if (prev === null || prev <= 1) {
            clearInterval(timer);
            return null;
          }
          return prev - 1;
        });
      }, 1000);

      return () => clearInterval(timer);
    }
  }, [error.code, error.retryAfter]);

  const colorClass = getErrorColorClass(error.code);
  const canRetry =
    error.retryable && !isRetrying && (countdown === null || countdown === 0);

  return (
    <div
      className={`flex items-start gap-3 p-3 rounded-lg border ${colorClass}`}
    >
      {/* 错误图标 */}
      <div className="flex-shrink-0 mt-0.5">{getErrorIcon(error.code)}</div>

      {/* 错误内容 */}
      <div className="flex-1 min-w-0">
        {/* 错误消息 */}
        <p className="text-sm font-medium">{error.message}</p>

        {/* 倒计时提示 */}
        {countdown !== null && countdown > 0 && (
          <p className="text-xs mt-1 opacity-80">{countdown} 秒后可重试</p>
        )}

        {/* 详细信息（可选显示） */}
        {error.details && (
          <details className="mt-2">
            <summary className="text-xs cursor-pointer opacity-70 hover:opacity-100">
              查看详情
            </summary>
            <pre className="text-xs mt-1 p-2 bg-black/5 rounded overflow-x-auto whitespace-pre-wrap break-all">
              {error.details}
            </pre>
          </details>
        )}
      </div>

      {/* 重试按钮 */}
      {error.retryable && onRetry && (
        <button
          onClick={onRetry}
          disabled={!canRetry}
          className={`
            flex-shrink-0 flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium
            transition-all duration-200
            ${
              canRetry
                ? "bg-white/80 hover:bg-white shadow-sm cursor-pointer"
                : "bg-white/40 cursor-not-allowed opacity-50"
            }
          `}
          title={
            canRetry
              ? "点击重试"
              : countdown
                ? `${countdown}秒后可重试`
                : "正在重试..."
          }
        >
          <RefreshCw
            className={`w-4 h-4 ${isRetrying ? "animate-spin" : ""}`}
          />
          {isRetrying ? "重试中..." : "重试"}
        </button>
      )}
    </div>
  );
};

export default ErrorDisplay;
