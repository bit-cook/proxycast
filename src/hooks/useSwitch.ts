import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import {
  switchApi,
  Provider,
  AppType,
  SyncCheckResult,
} from "@/lib/api/switch";

export function useSwitch(appType: AppType) {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [currentProvider, setCurrentProvider] = useState<Provider | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchProviders = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [list, current] = await Promise.all([
        switchApi.getProviders(appType),
        switchApi.getCurrentProvider(appType),
      ]);
      setProviders(list);
      setCurrentProvider(current);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [appType]);

  useEffect(() => {
    fetchProviders();
  }, [fetchProviders]);

  const addProvider = async (
    provider: Omit<Provider, "id" | "is_current" | "created_at">,
  ) => {
    const newProvider: Provider = {
      ...provider,
      id: crypto.randomUUID(),
      app_type: appType,
      is_current: false,
      created_at: Date.now(),
    };
    await switchApi.addProvider(newProvider);
    await fetchProviders();
    toast.success("配置已添加");
  };

  const updateProvider = async (provider: Provider) => {
    await switchApi.updateProvider(provider);
    await fetchProviders();
    toast.success("配置已更新");
  };

  const deleteProvider = async (id: string) => {
    await switchApi.deleteProvider(appType, id);
    await fetchProviders();
    toast.success("配置已删除");
  };

  const switchToProvider = async (id: string) => {
    await switchApi.switchProvider(appType, id);
    await fetchProviders();
    toast.success("切换成功");
  };

  const checkConfigSync = async (): Promise<SyncCheckResult> => {
    try {
      const result = await switchApi.checkConfigSync(appType);
      return result;
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      toast.error("检查同步状态失败: " + message);
      throw e;
    }
  };

  const syncFromExternal = async (): Promise<void> => {
    try {
      const message = await switchApi.syncFromExternal(appType);
      await fetchProviders(); // 刷新数据
      toast.success(message);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      toast.error("同步失败: " + message);
      throw e;
    }
  };

  return {
    providers,
    currentProvider,
    loading,
    error,
    addProvider,
    updateProvider,
    deleteProvider,
    switchToProvider,
    refresh: fetchProviders,
    checkConfigSync,
    syncFromExternal,
  };
}
