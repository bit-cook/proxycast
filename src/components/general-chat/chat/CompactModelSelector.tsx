/**
 * @file CompactModelSelector.tsx
 * @description 紧凑型模型选择器组件 - 用于 General Chat 输入栏上方
 * @module components/general-chat/chat/CompactModelSelector
 */

import React from "react";
import { AlertCircle, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";
import { ModelSelector } from "@/components/input-kit";

export interface CompactModelSelectorProps {
  /** 自定义类名 */
  className?: string;
  /** 是否禁用 */
  disabled?: boolean;
  /** 当前 provider 类型 */
  providerType: string;
  /** 当前模型 */
  model: string;
  /** 切换 provider */
  setProviderType: (providerType: string) => void;
  /** 切换模型 */
  setModel: (model: string) => void;
  /** provider 是否可用 */
  hasAvailableProvider: boolean;
  /** provider 是否加载中 */
  isLoading?: boolean;
  /** 错误信息 */
  error?: string | null;
  /** 跳转到 Provider 配置 */
  onManageProviders?: () => void;
}

export const CompactModelSelector: React.FC<CompactModelSelectorProps> = ({
  className,
  disabled = false,
  providerType,
  model,
  setProviderType,
  setModel,
  hasAvailableProvider,
  isLoading = false,
  error = null,
  onManageProviders,
}) => {

  if (!hasAvailableProvider && !isLoading) {
    return (
      <div
        className={cn(
          "w-full rounded-lg border border-amber-200 bg-amber-50/60 px-3 py-2.5",
          className,
        )}
      >
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-start gap-2 min-w-0">
            <AlertTriangle className="h-4 w-4 text-amber-600 mt-0.5 shrink-0" />
            <div className="min-w-0">
              <div className="text-sm font-medium text-amber-900">
                工具模型未配置
              </div>
              <div className="text-xs text-amber-700 leading-5">
                配置工具模型以获得更好的对话标题和记忆管理。
              </div>
            </div>
          </div>
          {onManageProviders && (
            <button
              type="button"
              onClick={onManageProviders}
              className="h-8 px-3 rounded-md border border-amber-300 bg-white text-xs font-medium text-amber-800 hover:bg-amber-100 hover:text-amber-900 transition-colors shrink-0"
            >
              配置
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className={cn("flex items-center gap-2", className)}>
      <ModelSelector
        providerType={providerType}
        setProviderType={setProviderType}
        model={model}
        setModel={setModel}
        compactTrigger
        popoverSide="top"
        disabled={disabled || isLoading}
        onManageProviders={onManageProviders}
      />
      {error && (
        <span className="inline-flex items-center gap-1 text-xs text-destructive">
          <AlertCircle size={12} />
          模型加载异常
        </span>
      )}
    </div>
  );
};

export default CompactModelSelector;
