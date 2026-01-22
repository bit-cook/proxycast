/**
 * @file useProvider.ts
 * @description Provider 选择逻辑 Hook
 * @module components/general-chat/hooks/useProvider
 *
 * 封装 Provider 选择、自动选择和故障切换逻辑
 * 复用现有的 useConfiguredProviders 和 useProviderModels
 *
 * @requirements 5.1, 5.2, 5.3
 */

import { useEffect, useCallback, useMemo, useRef } from "react";
import {
  useConfiguredProviders,
  type ConfiguredProvider,
} from "@/hooks/useConfiguredProviders";
import { useProviderModels } from "@/hooks/useProviderModels";
import { useGeneralChatStore } from "../store/useGeneralChatStore";
import type { ProviderConfig } from "../types";

// ============================================================================
// 类型定义
// ============================================================================

/**
 * useProvider Hook 返回值
 */
export interface UseProviderResult {
  /** 已配置的 Provider 列表 */
  providers: ConfiguredProvider[];
  /** 当前选中的 Provider */
  selectedProvider: ConfiguredProvider | null;
  /** 当前选中的模型 ID */
  selectedModelId: string | null;
  /** 可用的模型 ID 列表 */
  availableModelIds: string[];
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 是否有可用的 Provider */
  hasAvailableProvider: boolean;
  /** 当前的 Provider 配置（用于发送消息） */
  providerConfig: ProviderConfig | null;
  /** 选择 Provider */
  selectProvider: (providerKey: string) => void;
  /** 选择模型 */
  selectModel: (modelId: string) => void;
  /** 切换到下一个可用的 Provider（故障切换） */
  switchToNextProvider: () => boolean;
  /** 重置选择 */
  resetSelection: () => void;
}

// ============================================================================
// Hook 实现
// ============================================================================

/**
 * Provider 选择逻辑 Hook
 *
 * 功能：
 * 1. 复用现有 ProviderPool 系统（通过 useConfiguredProviders）
 * 2. 自动选择第一个可用的 Provider
 * 3. 实现故障切换逻辑
 * 4. 提供 Provider 配置给消息发送
 *
 * @returns Provider 选择相关的状态和操作
 *
 * @example
 * ```tsx
 * const {
 *   providers,
 *   selectedProvider,
 *   selectedModelId,
 *   providerConfig,
 *   selectProvider,
 *   selectModel,
 *   hasAvailableProvider,
 * } = useProvider();
 *
 * // 发送消息时使用 providerConfig
 * await sendMessage(content, providerConfig);
 * ```
 */
export function useProvider(): UseProviderResult {
  // 获取 Store 状态和操作
  const {
    providerSelection,
    setSelectedProvider,
    setSelectedModel,
    resetProviderSelection,
  } = useGeneralChatStore();

  const { selectedProviderKey, selectedModelId } = providerSelection;

  // 获取已配置的 Provider 列表（复用现有 Hook）
  const { providers, loading: providersLoading } = useConfiguredProviders();

  // 使用 ref 跟踪初始化状态，避免重复设置导致无限循环
  const providerInitializedRef = useRef(false);
  const modelInitializedRef = useRef(false);
  const lastProviderKeyRef = useRef<string | null>(null);

  // 获取当前选中的 Provider 对象
  const selectedProvider = useMemo(() => {
    if (!selectedProviderKey) return null;
    return providers.find((p) => p.key === selectedProviderKey) || null;
  }, [providers, selectedProviderKey]);

  // 获取选中 Provider 的模型列表（复用现有 Hook）
  const {
    modelIds: availableModelIds,
    loading: modelsLoading,
    error: modelsError,
  } = useProviderModels(selectedProvider);

  // 计算加载状态
  // 注意：只有在加载 Provider 列表时才显示加载状态
  // 模型加载是次要的，不应该阻塞整个界面
  const isLoading = providersLoading;

  // 计算错误状态
  const error = modelsError || providerSelection.providerError;

  // 是否有可用的 Provider
  const hasAvailableProvider = providers.length > 0;

  // ========== 自动选择逻辑 ==========

  // 自动选择第一个可用的 Provider（Requirements 5.2）
  // 使用 ref 确保只在必要时设置，避免无限循环
  useEffect(() => {
    if (providersLoading) return;

    // 如果没有选中的 Provider，且有可用的 Provider，自动选择第一个
    if (
      !selectedProviderKey &&
      providers.length > 0 &&
      !providerInitializedRef.current
    ) {
      providerInitializedRef.current = true;
      setSelectedProvider(providers[0].key);
      return;
    }

    // 如果选中的 Provider 不在列表中（可能被删除），重新选择
    if (
      selectedProviderKey &&
      !providers.find((p) => p.key === selectedProviderKey)
    ) {
      if (providers.length > 0) {
        setSelectedProvider(providers[0].key);
      } else {
        setSelectedProvider(null);
      }
    }
  }, [
    providersLoading,
    providers.length,
    selectedProviderKey,
    setSelectedProvider,
    providers,
  ]);

  // 当 Provider 切换时，重置模型初始化标记
  useEffect(() => {
    if (selectedProviderKey !== lastProviderKeyRef.current) {
      lastProviderKeyRef.current = selectedProviderKey;
      modelInitializedRef.current = false;
    }
  }, [selectedProviderKey]);

  // 自动选择第一个可用的模型
  useEffect(() => {
    if (modelsLoading) return;

    // 如果没有选中的模型，且有可用的模型，自动选择第一个
    if (
      !selectedModelId &&
      availableModelIds.length > 0 &&
      !modelInitializedRef.current
    ) {
      modelInitializedRef.current = true;
      setSelectedModel(availableModelIds[0]);
      return;
    }

    // 如果选中的模型不在列表中，重新选择
    if (selectedModelId && !availableModelIds.includes(selectedModelId)) {
      if (availableModelIds.length > 0) {
        setSelectedModel(availableModelIds[0]);
      } else {
        setSelectedModel(null);
      }
    }
  }, [modelsLoading, availableModelIds, selectedModelId, setSelectedModel]);

  // ========== 操作方法 ==========

  /**
   * 选择 Provider
   */
  const selectProvider = useCallback(
    (providerKey: string) => {
      const provider = providers.find((p) => p.key === providerKey);
      if (provider) {
        setSelectedProvider(providerKey);
        // 切换 Provider 时清除模型选择，让自动选择逻辑处理
        setSelectedModel(null);
      }
    },
    [providers, setSelectedProvider, setSelectedModel],
  );

  /**
   * 选择模型
   */
  const selectModel = useCallback(
    (modelId: string) => {
      if (availableModelIds.includes(modelId)) {
        setSelectedModel(modelId);
      }
    },
    [availableModelIds, setSelectedModel],
  );

  /**
   * 切换到下一个可用的 Provider（故障切换）
   * Requirements 5.3: 当前 Provider 不可用时自动切换
   *
   * @returns 是否成功切换
   */
  const switchToNextProvider = useCallback(() => {
    if (providers.length <= 1) {
      return false;
    }

    const currentIndex = providers.findIndex(
      (p) => p.key === selectedProviderKey,
    );
    const nextIndex = (currentIndex + 1) % providers.length;
    const nextProvider = providers[nextIndex];

    if (nextProvider && nextProvider.key !== selectedProviderKey) {
      setSelectedProvider(nextProvider.key);
      setSelectedModel(null);
      return true;
    }

    return false;
  }, [providers, selectedProviderKey, setSelectedProvider, setSelectedModel]);

  /**
   * 重置选择
   */
  const resetSelection = useCallback(() => {
    resetProviderSelection();
  }, [resetProviderSelection]);

  // ========== 构建 Provider 配置 ==========

  /**
   * 当前的 Provider 配置（用于发送消息）
   */
  const providerConfig: ProviderConfig | null = useMemo(() => {
    if (!selectedProvider || !selectedModelId) {
      return null;
    }

    return {
      providerName: selectedProvider.key,
      modelName: selectedModelId,
    };
  }, [selectedProvider, selectedModelId]);

  return {
    providers,
    selectedProvider,
    selectedModelId,
    availableModelIds,
    isLoading,
    error,
    hasAvailableProvider,
    providerConfig,
    selectProvider,
    selectModel,
    switchToNextProvider,
    resetSelection,
  };
}

export default useProvider;
