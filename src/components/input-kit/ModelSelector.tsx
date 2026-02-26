import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertCircle,
  ArrowRight,
  Bot,
  Check,
  ChevronDown,
  Settings2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { ProviderIcon } from "@/icons/providers";
import { useConfiguredProviders } from "@/hooks/useConfiguredProviders";
import { useProviderModels } from "@/hooks/useProviderModels";
import { providerPoolApi } from "@/lib/api/providerPool";
import { apiKeyProviderApi } from "@/lib/api/apiKeyProvider";
import { emitProviderDataChanged } from "@/lib/providerDataEvents";
import { filterModelsByTheme } from "@/components/agent/chat/utils/modelThemePolicy";

const THEME_LABEL_MAP: Record<string, string> = {
  general: "通用对话",
  "social-media": "社媒内容",
  poster: "图文海报",
  knowledge: "知识探索",
  planning: "计划规划",
  document: "办公文档",
  video: "短视频",
  music: "歌词曲谱",
  novel: "小说创作",
};

export interface ModelSelectorProps {
  providerType: string;
  setProviderType: (type: string) => void;
  model: string;
  setModel: (model: string) => void;
  activeTheme?: string;
  className?: string;
  compactTrigger?: boolean;
  onManageProviders?: () => void;
  popoverSide?: "top" | "bottom";
  disabled?: boolean;
}

export const ModelSelector: React.FC<ModelSelectorProps> = ({
  providerType,
  setProviderType,
  model,
  setModel,
  activeTheme,
  className,
  compactTrigger = false,
  onManageProviders,
  popoverSide = "top",
  disabled = false,
}) => {
  const [open, setOpen] = useState(false);
  const hasInitialized = useRef(false);
  const modelRef = useRef(model);
  modelRef.current = model;

  const { providers: configuredProviders, loading: providersLoading } =
    useConfiguredProviders();

  const selectedProvider = useMemo(() => {
    return configuredProviders.find(
      (provider) => provider.key === providerType,
    );
  }, [configuredProviders, providerType]);

  const { models: providerModels, loading: modelsLoading } = useProviderModels(
    selectedProvider,
    { returnFullMetadata: true },
  );

  const filteredResult = useMemo(() => {
    return filterModelsByTheme(activeTheme, providerModels);
  }, [activeTheme, providerModels]);

  const currentModels = useMemo(() => {
    return filteredResult.models.map((item) => item.id);
  }, [filteredResult.models]);

  useEffect(() => {
    if (hasInitialized.current) return;
    if (providersLoading) return;
    if (configuredProviders.length === 0) return;

    hasInitialized.current = true;

    if (!providerType.trim()) {
      setProviderType(configuredProviders[0].key);
    }
  }, [configuredProviders, providerType, providersLoading, setProviderType]);

  useEffect(() => {
    if (!selectedProvider) return;
    if (modelsLoading) return;

    const currentModel = modelRef.current;
    if (
      currentModels.length > 0 &&
      (!currentModel || !currentModels.includes(currentModel))
    ) {
      setModel(currentModels[0]);
    }
  }, [currentModels, modelsLoading, selectedProvider, setModel]);

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    if (!selectedProvider) return;
    if (!activeTheme) return;
    if (!filteredResult.usedFallback && filteredResult.filteredOutCount === 0) {
      return;
    }

    console.debug("[ModelSelector] 主题模型过滤结果", {
      theme: activeTheme,
      provider: selectedProvider.key,
      policyName: filteredResult.policyName,
      filteredOutCount: filteredResult.filteredOutCount,
      usedFallback: filteredResult.usedFallback,
    });
  }, [
    activeTheme,
    filteredResult.filteredOutCount,
    filteredResult.policyName,
    filteredResult.usedFallback,
    selectedProvider,
  ]);

  useEffect(() => {
    if (!open) return;
    if (disabled) return;

    let cancelled = false;

    const refreshProviderData = async () => {
      try {
        await Promise.all([
          providerPoolApi.getOverview(),
          apiKeyProviderApi.getProviders(),
        ]);

        if (!cancelled) {
          emitProviderDataChanged("provider_pool");
        }
      } catch (error) {
        console.error("[ModelSelector] 刷新 Provider 数据失败:", error);
      }
    };

    void refreshProviderData();

    return () => {
      cancelled = true;
    };
  }, [disabled, open]);

  useEffect(() => {
    if (!disabled) return;
    if (!open) return;
    setOpen(false);
  }, [disabled, open]);

  const selectedProviderLabel = selectedProvider?.label || providerType;
  const compactProviderType =
    selectedProvider?.key || providerType || "proxycast-hub";
  const compactProviderLabel =
    selectedProvider?.label || providerType || "ProxyCast Hub";
  const normalizedTheme = (activeTheme || "").toLowerCase();
  const activeThemeLabel =
    THEME_LABEL_MAP[normalizedTheme] || activeTheme || "当前主题";
  const showThemeFilterHint =
    normalizedTheme !== "" &&
    normalizedTheme !== "general" &&
    !filteredResult.usedFallback &&
    filteredResult.filteredOutCount > 0;
  const showNoProviderGuide =
    !providersLoading && configuredProviders.length === 0;

  if (showNoProviderGuide) {
    return (
      <div
        className={cn(
          "w-full rounded-lg border border-amber-200 bg-amber-50/60 p-3",
          className,
        )}
      >
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-start gap-2 min-w-0">
            <AlertCircle className="h-4 w-4 text-amber-600 mt-0.5 shrink-0" />
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
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0 border-amber-300 bg-white text-amber-800 hover:bg-amber-100 hover:text-amber-900"
              onClick={onManageProviders}
            >
              配置
            </Button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className={cn("flex items-center", className)}>
      <Popover
        open={open}
        onOpenChange={(nextOpen) => {
          if (disabled) {
            return;
          }
          setOpen(nextOpen);
        }}
      >
        <PopoverTrigger asChild>
          {compactTrigger ? (
            <Button
              variant="ghost"
              size="icon"
              role="combobox"
              aria-expanded={open}
              disabled={disabled}
              className={cn(
                "h-[30px] w-[30px] rounded-full p-0 text-muted-foreground",
                "hover:bg-secondary hover:text-foreground",
                open && "bg-secondary text-foreground",
              )}
              title={`${selectedProviderLabel} / ${model || "选择模型"}`}
            >
              <ProviderIcon
                providerType={compactProviderType}
                fallbackText={compactProviderLabel}
                size={15}
              />
            </Button>
          ) : (
            <Button
              variant="outline"
              role="combobox"
              aria-expanded={open}
              disabled={disabled}
              className="h-9 px-3 gap-2 font-normal bg-background hover:bg-muted/60"
            >
              <Bot size={16} className="text-primary" />
              <span className="font-medium truncate max-w-[160px]">
                {selectedProviderLabel}
              </span>
              <span className="text-muted-foreground">/</span>
              <span className="text-sm text-muted-foreground truncate max-w-[180px]">
                {model || "选择模型"}
              </span>
              <ChevronDown className="ml-1 h-3 w-3 text-muted-foreground opacity-50" />
            </Button>
          )}
        </PopoverTrigger>

        <PopoverContent
          className="w-[420px] p-0 bg-background/95 backdrop-blur-sm border-border shadow-lg"
          align="start"
          side={popoverSide}
          sideOffset={8}
          avoidCollisions
        >
          <div className="flex h-[320px]">
            <div className="w-[140px] border-r bg-muted/30 p-2 flex flex-col gap-1 overflow-y-auto">
              <div className="text-xs font-semibold text-muted-foreground px-2 py-1.5 mb-1">
                Providers
              </div>

              {configuredProviders.length === 0 ? (
                <div className="text-xs text-muted-foreground p-2">
                  暂无已配置的 Provider
                </div>
              ) : (
                configuredProviders.map((provider) => {
                  const isSelected = providerType === provider.key;

                  return (
                    <button
                      key={provider.key}
                      onClick={() => setProviderType(provider.key)}
                      className={cn(
                        "flex items-center justify-between w-full px-2 py-1.5 text-sm rounded-md transition-colors text-left",
                        isSelected
                          ? "bg-primary/10 text-primary font-medium"
                          : "hover:bg-muted text-muted-foreground hover:text-foreground",
                      )}
                    >
                      <span className="flex items-center gap-2 min-w-0">
                        <ProviderIcon
                          providerType={provider.key}
                          fallbackText={provider.label}
                          size={15}
                        />
                        <span className="truncate">{provider.label}</span>
                      </span>
                      {isSelected && (
                        <div className="w-1 h-1 rounded-full bg-primary" />
                      )}
                    </button>
                  );
                })
              )}
            </div>

            <div className="flex-1 p-2 flex flex-col overflow-hidden">
              <div className="text-xs font-semibold text-muted-foreground px-2 py-1.5 mb-1">
                Models
              </div>
              {showThemeFilterHint && (
                <div className="text-[11px] text-muted-foreground px-2 pb-1">
                  已按 {activeThemeLabel} 主题筛选模型
                </div>
              )}
              {normalizedTheme !== "general" && filteredResult.usedFallback && (
                <div className="text-[11px] text-amber-600 px-2 pb-1">
                  {activeThemeLabel} 未匹配到主题模型，已展示全部模型
                </div>
              )}

              <ScrollArea className="flex-1">
                <div className="space-y-1 p-1">
                  {currentModels.length === 0 ? (
                    <div className="text-xs text-muted-foreground p-2">
                      暂无可用模型
                    </div>
                  ) : (
                    currentModels.map((currentModelItem) => (
                      <button
                        key={currentModelItem}
                        onClick={() => {
                          setModel(currentModelItem);
                          setOpen(false);
                        }}
                        className={cn(
                          "flex items-center justify-between w-full px-2 py-1.5 text-sm rounded-md transition-colors text-left group",
                          model === currentModelItem
                            ? "bg-accent text-accent-foreground"
                            : "hover:bg-muted text-muted-foreground hover:text-foreground",
                        )}
                      >
                        <span className="flex items-center gap-2 min-w-0">
                          {selectedProvider && (
                            <ProviderIcon
                              providerType={selectedProvider.key}
                              fallbackText={selectedProvider.label}
                              size={15}
                            />
                          )}
                          <span className="truncate">{currentModelItem}</span>
                        </span>
                        {model === currentModelItem && (
                          <Check size={14} className="text-primary" />
                        )}
                      </button>
                    ))
                  )}
                </div>
              </ScrollArea>
            </div>
          </div>

          {onManageProviders && (
            <button
              type="button"
              className="w-full h-11 px-3 border-t flex items-center justify-between text-sm hover:bg-muted/60 transition-colors"
              onClick={() => {
                setOpen(false);
                onManageProviders();
              }}
            >
              <span className="inline-flex items-center gap-2 text-foreground">
                <Settings2 size={14} className="text-muted-foreground" />
                管理供应商
              </span>
              <ArrowRight size={14} className="text-muted-foreground" />
            </button>
          )}
        </PopoverContent>
      </Popover>
    </div>
  );
};
