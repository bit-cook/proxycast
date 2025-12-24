import { useState } from "react";
import { AlertTriangle, CheckCircle, XCircle, RefreshCw } from "lucide-react";
import { SyncCheckResult, SyncStatus } from "@/lib/api/switch";

interface ConfigSyncDialogProps {
  isOpen: boolean;
  syncResult: SyncCheckResult | null;
  onClose: () => void;
  onSyncFromExternal: () => Promise<void>;
  onRefreshCheck: () => Promise<void>;
}

export function ConfigSyncDialog({
  isOpen,
  syncResult,
  onClose,
  onSyncFromExternal,
  onRefreshCheck,
}: ConfigSyncDialogProps) {
  const [syncing, setSyncing] = useState(false);
  const [checking, setChecking] = useState(false);

  if (!isOpen || !syncResult) return null;

  const getSyncStatusIcon = (status: SyncStatus) => {
    switch (status) {
      case "InSync":
        return <CheckCircle className="h-5 w-5 text-green-500" />;
      case "OutOfSync":
        return <AlertTriangle className="h-5 w-5 text-yellow-500" />;
      case "Conflict":
        return <XCircle className="h-5 w-5 text-red-500" />;
    }
  };

  const getSyncStatusText = (status: SyncStatus) => {
    switch (status) {
      case "InSync":
        return "配置已同步";
      case "OutOfSync":
        return "配置有差异";
      case "Conflict":
        return "配置冲突";
    }
  };

  const getSyncStatusColor = (status: SyncStatus) => {
    switch (status) {
      case "InSync":
        return "text-green-700 bg-green-50 border-green-200";
      case "OutOfSync":
        return "text-yellow-700 bg-yellow-50 border-yellow-200";
      case "Conflict":
        return "text-red-700 bg-red-50 border-red-200";
    }
  };

  const handleSyncFromExternal = async () => {
    setSyncing(true);
    try {
      await onSyncFromExternal();
      onClose();
    } catch (_e) {
      // Error is handled in the hook
    } finally {
      setSyncing(false);
    }
  };

  const handleRefreshCheck = async () => {
    setChecking(true);
    try {
      await onRefreshCheck();
    } catch (_e) {
      // Error is handled in the hook
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg max-w-md w-full mx-4">
        <div className="p-6">
          <div className="flex items-center gap-3 mb-4">
            {getSyncStatusIcon(syncResult.status)}
            <h3 className="text-lg font-semibold">配置同步状态</h3>
          </div>

          <div
            className={`rounded-lg border p-4 mb-4 ${getSyncStatusColor(syncResult.status)}`}
          >
            <div className="flex items-center gap-2 mb-2">
              {getSyncStatusIcon(syncResult.status)}
              <span className="font-medium">
                {getSyncStatusText(syncResult.status)}
              </span>
            </div>

            <div className="space-y-2 text-sm">
              <div>
                <span className="font-medium">ProxyCast 当前配置:</span>{" "}
                {syncResult.current_provider}
              </div>
              <div>
                <span className="font-medium">外部软件当前配置:</span>{" "}
                {syncResult.external_provider}
              </div>
              {syncResult.last_modified && (
                <div>
                  <span className="font-medium">配置文件修改时间:</span>{" "}
                  {new Date(
                    parseInt(syncResult.last_modified) * 1000,
                  ).toLocaleString()}
                </div>
              )}
            </div>

            {syncResult.conflicts.length > 0 && (
              <div className="mt-3 pt-3 border-t border-current/20">
                <div className="font-medium mb-2">冲突详情:</div>
                {syncResult.conflicts.map((conflict, index) => (
                  <div key={index} className="text-sm space-y-1">
                    <div>字段: {conflict.field}</div>
                    <div>ProxyCast: {conflict.local_value}</div>
                    <div>外部软件: {conflict.external_value}</div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {syncResult.status !== "InSync" && (
            <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <p className="text-sm text-blue-700 dark:text-blue-400">
                {syncResult.status === "OutOfSync" && (
                  <>
                    检测到外部软件的配置与 ProxyCast
                    不同。您可以选择同步外部配置到 ProxyCast。
                  </>
                )}
                {syncResult.status === "Conflict" && (
                  <>
                    检测到配置冲突。建议选择使用外部软件的配置，或者手动在
                    ProxyCast 中重新设置。
                  </>
                )}
              </p>
            </div>
          )}

          <div className="flex gap-2 justify-end">
            <button
              onClick={handleRefreshCheck}
              disabled={checking || syncing}
              className="px-3 py-2 text-sm border rounded-lg hover:bg-muted disabled:opacity-50 flex items-center gap-2"
            >
              <RefreshCw
                className={`h-4 w-4 ${checking ? "animate-spin" : ""}`}
              />
              重新检查
            </button>

            {syncResult.status !== "InSync" && (
              <button
                onClick={handleSyncFromExternal}
                disabled={syncing || checking}
                className="px-3 py-2 text-sm bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 flex items-center gap-2"
              >
                {syncing && <RefreshCw className="h-4 w-4 animate-spin" />}
                使用外部配置
              </button>
            )}

            <button
              onClick={onClose}
              disabled={syncing || checking}
              className="px-3 py-2 text-sm border rounded-lg hover:bg-muted disabled:opacity-50"
            >
              关闭
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
