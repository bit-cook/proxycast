/**
 * @file ErrorBoundary.tsx
 * @description Error Boundary 组件 - 捕获渲染错误并显示降级 UI
 * @module components/general-chat/chat/ErrorBoundary
 *
 * 使用 React 的 Error Boundary 模式捕获子组件的渲染错误，
 * 记录错误日志并显示友好的错误提示界面
 *
 * @requirements 9.4
 */

import React, { Component, ErrorInfo, ReactNode } from "react";
import { AlertTriangle, RefreshCw, Home, Bug } from "lucide-react";

// ============================================================================
// 类型定义
// ============================================================================

interface ErrorBoundaryProps {
  /** 子组件 */
  children: ReactNode;
  /** 自定义降级 UI 渲染函数 */
  fallback?: (error: Error, resetError: () => void) => ReactNode;
  /** 错误发生时的回调 */
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
  /** 组件名称，用于日志记录 */
  componentName?: string;
}

interface ErrorBoundaryState {
  /** 是否发生错误 */
  hasError: boolean;
  /** 错误对象 */
  error: Error | null;
  /** 错误信息 */
  errorInfo: ErrorInfo | null;
}

// ============================================================================
// Error Boundary 组件
// ============================================================================

/**
 * Error Boundary 组件
 *
 * 捕获子组件树中的 JavaScript 错误，记录错误日志，
 * 并显示降级 UI 而不是崩溃的组件树
 *
 * @example
 * ```tsx
 * <ErrorBoundary componentName="ChatPanel">
 *   <ChatPanel />
 * </ErrorBoundary>
 * ```
 *
 * @requirements 9.4
 */
export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  /**
   * 从错误中派生状态
   * 当子组件抛出错误时，React 会调用此方法
   */
  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      hasError: true,
      error,
    };
  }

  /**
   * 捕获错误并记录日志
   * 在渲染阶段、生命周期方法和整个子组件树的构造函数中捕获错误
   */
  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    const { onError, componentName } = this.props;

    // 更新状态以包含错误信息
    this.setState({ errorInfo });

    // 记录错误日志
    const logContext = componentName ? `[${componentName}]` : "[ErrorBoundary]";
    console.error(`${logContext} 捕获到渲染错误:`, error);
    console.error(`${logContext} 组件堆栈:`, errorInfo.componentStack);

    // 调用外部错误处理回调
    if (onError) {
      onError(error, errorInfo);
    }

    // 可以在这里添加错误上报逻辑
    // 例如：发送到错误监控服务
    this.reportError(error, errorInfo);
  }

  /**
   * 上报错误到监控服务（预留接口）
   */
  private reportError(error: Error, errorInfo: ErrorInfo): void {
    // TODO: 集成错误监控服务（如 Sentry）
    // 目前仅在控制台输出
    const errorReport = {
      name: error.name,
      message: error.message,
      stack: error.stack,
      componentStack: errorInfo.componentStack,
      timestamp: new Date().toISOString(),
      componentName: this.props.componentName,
    };

    console.info("[ErrorBoundary] 错误报告:", errorReport);
  }

  /**
   * 重置错误状态
   */
  private resetError = (): void => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  /**
   * 刷新页面
   */
  private handleRefresh = (): void => {
    window.location.reload();
  };

  /**
   * 返回首页
   */
  private handleGoHome = (): void => {
    window.location.href = "/";
  };

  render(): ReactNode {
    const { hasError, error, errorInfo } = this.state;
    const { children, fallback } = this.props;

    if (hasError && error) {
      // 如果提供了自定义降级 UI，使用它
      if (fallback) {
        return fallback(error, this.resetError);
      }

      // 默认降级 UI
      return (
        <DefaultFallbackUI
          error={error}
          errorInfo={errorInfo}
          onRetry={this.resetError}
          onRefresh={this.handleRefresh}
          onGoHome={this.handleGoHome}
        />
      );
    }

    return children;
  }
}

// ============================================================================
// 默认降级 UI 组件
// ============================================================================

interface DefaultFallbackUIProps {
  error: Error;
  errorInfo: ErrorInfo | null;
  onRetry: () => void;
  onRefresh: () => void;
  onGoHome: () => void;
}

/**
 * 默认降级 UI
 *
 * 显示友好的错误提示和操作按钮
 */
const DefaultFallbackUI: React.FC<DefaultFallbackUIProps> = ({
  error,
  errorInfo,
  onRetry,
  onRefresh,
  onGoHome,
}) => {
  const [showDetails, setShowDetails] = React.useState(false);

  return (
    <div className="flex flex-col items-center justify-center h-full min-h-[400px] p-8 bg-surface">
      {/* 错误图标 */}
      <div className="w-20 h-20 mb-6 rounded-full bg-red-100 flex items-center justify-center">
        <AlertTriangle className="w-10 h-10 text-red-600" />
      </div>

      {/* 错误标题 */}
      <h2 className="text-xl font-semibold text-ink-900 mb-3">内容渲染失败</h2>

      {/* 错误描述 */}
      <p className="text-sm text-ink-500 text-center max-w-md mb-6 leading-relaxed">
        抱歉，页面遇到了一些问题。您可以尝试重试或刷新页面。
        如果问题持续存在，请联系技术支持。
      </p>

      {/* 操作按钮 */}
      <div className="flex items-center gap-3 mb-6">
        <button
          type="button"
          onClick={onRetry}
          className="inline-flex items-center gap-2 px-4 py-2 bg-accent text-white rounded-lg font-medium hover:bg-accent/90 transition-colors shadow-sm"
        >
          <RefreshCw className="w-4 h-4" />
          重试
        </button>

        <button
          type="button"
          onClick={onRefresh}
          className="inline-flex items-center gap-2 px-4 py-2 bg-ink-100 text-ink-700 rounded-lg font-medium hover:bg-ink-200 transition-colors"
        >
          <RefreshCw className="w-4 h-4" />
          刷新页面
        </button>

        <button
          type="button"
          onClick={onGoHome}
          className="inline-flex items-center gap-2 px-4 py-2 bg-ink-100 text-ink-700 rounded-lg font-medium hover:bg-ink-200 transition-colors"
        >
          <Home className="w-4 h-4" />
          返回首页
        </button>
      </div>

      {/* 错误详情（可展开） */}
      <div className="w-full max-w-lg">
        <button
          type="button"
          onClick={() => setShowDetails(!showDetails)}
          className="flex items-center gap-2 text-xs text-ink-400 hover:text-ink-600 transition-colors mx-auto"
        >
          <Bug className="w-3 h-3" />
          {showDetails ? "隐藏错误详情" : "查看错误详情"}
        </button>

        {showDetails && (
          <div className="mt-4 p-4 bg-ink-50 rounded-lg border border-ink-200">
            <div className="mb-3">
              <p className="text-xs font-medium text-ink-600 mb-1">错误类型:</p>
              <p className="text-xs text-red-600 font-mono">{error.name}</p>
            </div>

            <div className="mb-3">
              <p className="text-xs font-medium text-ink-600 mb-1">错误信息:</p>
              <p className="text-xs text-red-600 font-mono break-all">
                {error.message}
              </p>
            </div>

            {error.stack && (
              <div className="mb-3">
                <p className="text-xs font-medium text-ink-600 mb-1">
                  堆栈跟踪:
                </p>
                <pre className="text-xs text-ink-500 font-mono overflow-x-auto whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
                  {error.stack}
                </pre>
              </div>
            )}

            {errorInfo?.componentStack && (
              <div>
                <p className="text-xs font-medium text-ink-600 mb-1">
                  组件堆栈:
                </p>
                <pre className="text-xs text-ink-500 font-mono overflow-x-auto whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
                  {errorInfo.componentStack}
                </pre>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default ErrorBoundary;
