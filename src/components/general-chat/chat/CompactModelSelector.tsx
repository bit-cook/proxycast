/**
 * @file CompactModelSelector.tsx
 * @description 紧凑型模型选择器组件 - 用于 General Chat 输入栏上方
 * @module components/general-chat/chat/CompactModelSelector
 *
 * 复用现有的 ProviderModelSelector 组件，提供紧凑的下拉选择体验
 *
 * @requirements 5.4
 */

import React, { useState, useRef, useEffect } from "react";
import { ChevronDown, Loader2, AlertCircle, Check } from "lucide-react";
import { cn } from "@/lib/utils";
import { useProvider } from "../hooks/useProvider";
import { getProviderLabel } from "@/lib/constants/providerMappings";

// ============================================================================
// 类型定义
// ============================================================================

export interface CompactModelSelectorProps {
  /** 自定义类名 */
  className?: string;
  /** 是否禁用 */
  disabled?: boolean;
}

// ============================================================================
// 子组件
// ============================================================================

interface DropdownMenuProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode;
}

/**
 * 下拉菜单容器
 */
const DropdownMenu: React.FC<DropdownMenuProps> = ({
  isOpen,
  onClose,
  children,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      ref={menuRef}
      className="absolute bottom-full left-0 mb-2 w-80 max-h-96 overflow-hidden rounded-lg border border-ink-200 bg-surface shadow-lg z-50"
    >
      {children}
    </div>
  );
};

// ============================================================================
// 主组件
// ============================================================================

/**
 * 紧凑型模型选择器组件
 *
 * 显示当前选中的 Provider 和模型，点击展开下拉菜单进行选择
 * 复用 useProvider hook 获取 Provider 和模型数据
 *
 * @example
 * ```tsx
 * <CompactModelSelector />
 * ```
 */
export const CompactModelSelector: React.FC<CompactModelSelectorProps> = ({
  className,
  disabled = false,
}) => {
  const [isOpen, setIsOpen] = useState(false);

  const {
    providers,
    selectedProvider,
    selectedModelId,
    availableModelIds,
    isLoading,
    error,
    hasAvailableProvider,
    selectProvider,
    selectModel,
  } = useProvider();

  // 获取显示文本
  const displayText = React.useMemo(() => {
    if (isLoading) {
      return "加载中...";
    }
    if (!hasAvailableProvider) {
      return "未配置 Provider";
    }
    if (!selectedProvider) {
      return "选择模型";
    }
    const providerLabel = getProviderLabel(selectedProvider.key);
    if (!selectedModelId) {
      return providerLabel;
    }
    // 简化模型名称显示
    const shortModelId = selectedModelId.split("/").pop() || selectedModelId;
    return `${providerLabel} / ${shortModelId}`;
  }, [isLoading, hasAvailableProvider, selectedProvider, selectedModelId]);

  // 处理 Provider 选择
  const handleProviderSelect = (providerKey: string) => {
    selectProvider(providerKey);
  };

  // 处理模型选择
  const handleModelSelect = (modelId: string) => {
    selectModel(modelId);
    setIsOpen(false);
  };

  // 无 Provider 时显示提示
  if (!hasAvailableProvider && !isLoading) {
    return (
      <div
        className={cn(
          "flex items-center gap-2 text-sm text-amber-600",
          className,
        )}
      >
        <AlertCircle className="h-4 w-4" />
        <span>请先配置 Provider 凭证</span>
      </div>
    );
  }

  return (
    <div className={cn("relative", className)}>
      {/* 触发按钮 */}
      <button
        type="button"
        onClick={() => !disabled && setIsOpen(!isOpen)}
        disabled={disabled || isLoading}
        className={cn(
          "flex items-center gap-2 px-3 py-1.5 text-sm rounded-md transition-colors",
          "border border-ink-200 hover:border-ink-300 hover:bg-ink-50",
          "focus:outline-none focus:ring-2 focus:ring-accent/50",
          disabled && "opacity-50 cursor-not-allowed",
          isOpen && "border-accent bg-accent/5",
        )}
      >
        {isLoading ? (
          <Loader2 className="h-4 w-4 animate-spin text-ink-400" />
        ) : error ? (
          <AlertCircle className="h-4 w-4 text-red-500" />
        ) : (
          <div className="w-2 h-2 rounded-full bg-green-500" />
        )}
        <span className="truncate max-w-[200px]">{displayText}</span>
        <ChevronDown
          className={cn(
            "h-4 w-4 text-ink-400 transition-transform",
            isOpen && "rotate-180",
          )}
        />
      </button>

      {/* 下拉菜单 */}
      <DropdownMenu isOpen={isOpen} onClose={() => setIsOpen(false)}>
        <div className="flex h-72">
          {/* 左侧：Provider 列表 */}
          <div className="w-32 border-r border-ink-200 bg-ink-50/50 flex flex-col">
            <div className="px-3 py-2 border-b border-ink-200 bg-ink-100/50">
              <h4 className="text-xs font-medium text-ink-600">Provider</h4>
            </div>
            <div className="flex-1 overflow-y-auto p-1">
              {providers.map((provider) => (
                <button
                  key={provider.key}
                  type="button"
                  onClick={() => handleProviderSelect(provider.key)}
                  className={cn(
                    "w-full flex items-center gap-2 px-2 py-1.5 text-xs rounded transition-colors",
                    selectedProvider?.key === provider.key
                      ? "bg-accent text-white"
                      : "hover:bg-ink-100 text-ink-700",
                  )}
                >
                  <span className="truncate">{provider.label}</span>
                </button>
              ))}
            </div>
          </div>

          {/* 右侧：模型列表 */}
          <div className="flex-1 flex flex-col min-w-0">
            <div className="px-3 py-2 border-b border-ink-200 bg-ink-100/50">
              <h4 className="text-xs font-medium text-ink-600">
                {selectedProvider
                  ? `${getProviderLabel(selectedProvider.key)} 模型`
                  : "请选择 Provider"}
              </h4>
            </div>
            <div className="flex-1 overflow-y-auto p-1">
              {availableModelIds.length === 0 ? (
                <div className="flex items-center justify-center h-full text-xs text-ink-400">
                  暂无可用模型
                </div>
              ) : (
                availableModelIds.map((modelId) => {
                  const isSelected = selectedModelId === modelId;
                  // 简化模型名称显示
                  const displayName = modelId.split("/").pop() || modelId;

                  return (
                    <button
                      key={modelId}
                      type="button"
                      onClick={() => handleModelSelect(modelId)}
                      className={cn(
                        "w-full flex items-center justify-between px-2 py-1.5 text-xs rounded transition-colors",
                        isSelected
                          ? "bg-accent/10 text-accent border border-accent/30"
                          : "hover:bg-ink-100 text-ink-700 border border-transparent",
                      )}
                    >
                      <span className="truncate">{displayName}</span>
                      {isSelected && (
                        <Check className="h-3 w-3 flex-shrink-0" />
                      )}
                    </button>
                  );
                })
              )}
            </div>
          </div>
        </div>
      </DropdownMenu>
    </div>
  );
};

export default CompactModelSelector;
