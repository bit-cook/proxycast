import { useState } from "react";
import { Plus, RefreshCw, Eye, GitCompare } from "lucide-react";
import { AppType, SyncCheckResult } from "@/lib/api/switch";
import { useSwitch } from "@/hooks/useSwitch";
import { ProviderCard } from "./ProviderCard";
import { ProviderForm } from "./ProviderForm";
import { LiveConfigModal } from "./LiveConfigModal";
import { ConfigSyncDialog } from "./ConfigSyncDialog";
import { ConfirmDialog } from "@/components/ConfirmDialog";

interface ProviderListProps {
  appType: AppType;
}

export function ProviderList({ appType }: ProviderListProps) {
  const {
    providers,
    currentProvider,
    loading,
    error,
    addProvider,
    updateProvider,
    deleteProvider,
    switchToProvider,
    refresh,
    checkConfigSync,
    syncFromExternal,
  } = useSwitch(appType);

  const [showForm, setShowForm] = useState(false);
  const [editingProvider, setEditingProvider] = useState<
    (typeof providers)[0] | null
  >(null);
  const [showLiveConfig, setShowLiveConfig] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [showSyncDialog, setShowSyncDialog] = useState(false);
  const [syncResult, setSyncResult] = useState<SyncCheckResult | null>(null);
  const [checkingSync, setCheckingSync] = useState(false);

  const handleAdd = () => {
    setEditingProvider(null);
    setShowForm(true);
  };

  const handleEdit = (provider: (typeof providers)[0]) => {
    setEditingProvider(provider);
    setShowForm(true);
  };

  const handleSave = async (data: Parameters<typeof addProvider>[0]) => {
    if (editingProvider) {
      await updateProvider({ ...editingProvider, ...data });
    } else {
      await addProvider(data);
    }
    setShowForm(false);
    setEditingProvider(null);
  };

  const handleDeleteClick = (id: string) => {
    // 不能删除当前使用中的 provider
    if (currentProvider?.id === id) {
      alert("无法删除当前使用中的配置");
      return;
    }
    setDeleteConfirm(id);
  };

  const handleDeleteConfirm = async () => {
    if (!deleteConfirm) return;
    try {
      await deleteProvider(deleteConfirm);
    } catch (e) {
      alert("删除失败: " + (e instanceof Error ? e.message : String(e)));
    } finally {
      setDeleteConfirm(null);
    }
  };

  const handleCheckSync = async () => {
    setCheckingSync(true);
    try {
      const result = await checkConfigSync();
      setSyncResult(result);
      setShowSyncDialog(true);
    } catch (_e) {
      // Error is handled in the hook
    } finally {
      setCheckingSync(false);
    }
  };

  const handleSyncFromExternal = async () => {
    await syncFromExternal();
    // 重新检查同步状态
    const result = await checkConfigSync();
    setSyncResult(result);
  };

  const handleRefreshSyncCheck = async () => {
    const result = await checkConfigSync();
    setSyncResult(result);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-lg border border-destructive bg-destructive/10 p-4">
        <p className="text-destructive">{error}</p>
        <button
          onClick={refresh}
          className="mt-2 text-sm text-muted-foreground hover:underline"
        >
          重试
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <p className="text-sm text-muted-foreground">
            当前: {currentProvider?.name || "未设置"}
          </p>
          <button
            onClick={() => setShowLiveConfig(true)}
            className="p-1.5 rounded hover:bg-muted text-muted-foreground hover:text-foreground"
            title="查看当前生效的配置"
          >
            <Eye className="h-4 w-4" />
          </button>
        </div>
        <div className="flex gap-2">
          <button
            onClick={handleCheckSync}
            disabled={checkingSync}
            className="p-2 rounded-lg hover:bg-muted"
            title="检查外部配置同步状态"
          >
            <GitCompare
              className={`h-4 w-4 ${checkingSync ? "animate-pulse" : ""}`}
            />
          </button>
          <button
            onClick={refresh}
            className="p-2 rounded-lg hover:bg-muted"
            title="刷新"
          >
            <RefreshCw className="h-4 w-4" />
          </button>
          <button
            onClick={handleAdd}
            className="flex items-center gap-2 px-3 py-2 rounded-lg bg-primary text-primary-foreground text-sm"
          >
            <Plus className="h-4 w-4" />
            添加 Provider
          </button>
        </div>
      </div>

      {providers.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <p>暂无 Provider 配置</p>
          <p className="text-sm mt-1">点击上方按钮添加第一个配置</p>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {providers.map((provider) => (
            <ProviderCard
              key={provider.id}
              provider={provider}
              isCurrent={provider.id === currentProvider?.id}
              onSwitch={() => switchToProvider(provider.id)}
              onEdit={() => handleEdit(provider)}
              onDelete={() => handleDeleteClick(provider.id)}
            />
          ))}
        </div>
      )}

      {showForm && (
        <ProviderForm
          appType={appType}
          provider={editingProvider}
          onSave={handleSave}
          onCancel={() => {
            setShowForm(false);
            setEditingProvider(null);
          }}
        />
      )}

      {showLiveConfig && (
        <LiveConfigModal
          appType={appType}
          onClose={() => setShowLiveConfig(false)}
        />
      )}

      <ConfirmDialog
        isOpen={!!deleteConfirm}
        title="删除确认"
        message="确定要删除这个 Provider 吗？"
        onConfirm={handleDeleteConfirm}
        onCancel={() => setDeleteConfirm(null)}
      />

      <ConfigSyncDialog
        isOpen={showSyncDialog}
        syncResult={syncResult}
        onClose={() => setShowSyncDialog(false)}
        onSyncFromExternal={handleSyncFromExternal}
        onRefreshCheck={handleRefreshSyncCheck}
      />
    </div>
  );
}
