import React, { useState, useMemo, useEffect, useRef } from "react";
import { ArrowRight, Bot, Check, ChevronDown, Settings2 } from "lucide-react";
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
import { isAliasProvider } from "@/lib/constants/providerMappings";
import { providerPoolApi } from "@/lib/api/providerPool";
import { apiKeyProviderApi } from "@/lib/api/apiKeyProvider";
import { emitProviderDataChanged } from "@/lib/providerDataEvents";

interface ChatModelSelectorProps {
  providerType: string;
  setProviderType: (type: string) => void;
  model: string;
  setModel: (model: string) => void;
  className?: string;
  compactTrigger?: boolean;
  onManageProviders?: () => void;
  popoverSide?: "top" | "bottom";
}

export const ChatModelSelector: React.FC<ChatModelSelectorProps> = ({
  providerType,
  setProviderType,
  model,
  setModel,
  className,
  compactTrigger = false,
  onManageProviders,
  popoverSide = "top",
}) => {
  const [open, setOpen] = useState(false);
  const hasInitialized = useRef(false);
  const modelRef = useRef(model);
  modelRef.current = model;

  const { providers: configuredProviders } = useConfiguredProviders();

  const selectedProvider = useMemo(() => {
    return configuredProviders.find(
      (provider) => provider.key === providerType,
    );
  }, [configuredProviders, providerType]);

  const { modelIds: currentModels, loading: modelsLoading } =
    useProviderModels(selectedProvider);

  useEffect(() => {
    if (hasInitialized.current) return;
    if (configuredProviders.length === 0) return;

    hasInitialized.current = true;

    if (!selectedProvider) {
      setProviderType(configuredProviders[0].key);
    }
  }, [configuredProviders, selectedProvider, setProviderType]);

  useEffect(() => {
    if (
      selectedProvider &&
      isAliasProvider(selectedProvider.key) &&
      modelsLoading
    ) {
      return;
    }

    const currentModel = modelRef.current;
    if (
      currentModels.length > 0 &&
      (!currentModel || !currentModels.includes(currentModel))
    ) {
      setModel(currentModels[0]);
    }
  }, [currentModels, modelsLoading, selectedProvider, setModel]);

  useEffect(() => {
    if (!open) return;

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
        console.error("[ChatModelSelector] 刷新 Provider 数据失败:", error);
      }
    };

    void refreshProviderData();

    return () => {
      cancelled = true;
    };
  }, [open]);

  const selectedProviderLabel = selectedProvider?.label || providerType;
  const compactProviderType =
    selectedProvider?.key || providerType || "proxycast-hub";
  const compactProviderLabel =
    selectedProvider?.label || providerType || "ProxyCast Hub";

  return (
    <div className={cn("flex items-center", className)}>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          {compactTrigger ? (
            <Button
              variant="ghost"
              size="icon"
              role="combobox"
              aria-expanded={open}
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
