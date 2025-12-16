import { useState, useEffect } from "react";
import {
  RefreshCw,
  Save,
  RotateCcw,
  Trash2,
  ArrowRightLeft,
} from "lucide-react";
import {
  resilienceApi,
  type FailoverConfig,
  type SwitchLogEntry,
} from "@/lib/api/resilience";
import { HelpTip } from "@/components/HelpTip";

interface FailoverSettingsProps {
  onSave?: () => void;
}

export function FailoverSettings({ onSave }: FailoverSettingsProps) {
  const [config, setConfig] = useState<FailoverConfig>({
    auto_switch: true,
    switch_on_quota: true,
  });
  const [switchLog, setSwitchLog] = useState<SwitchLogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [hasChanges, setHasChanges] = useState(false);
  const [originalConfig, setOriginalConfig] = useState<FailoverConfig | null>(
    null,
  );

  const loadData = async () => {
    setLoading(true);
    setError(null);
    try {
      const [configData, logData] = await Promise.all([
        resilienceApi.getFailoverConfig(),
        resilienceApi.getSwitchLog(),
      ]);
      setConfig(configData);
      setOriginalConfig(configData);
      setSwitchLog(logData);
      setHasChanges(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSuccess(null);
    try {
      await resilienceApi.updateFailoverConfig(config);
      setOriginalConfig(config);
      setHasChanges(false);
      setSuccess("故障转移配置已保存");
      onSave?.();
      setTimeout(() => setSuccess(null), 3000);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    if (originalConfig) {
      setConfig(originalConfig);
      setHasChanges(false);
    }
  };

  const handleClearLog = async () => {
    try {
      await resilienceApi.clearSwitchLog();
      setSwitchLog([]);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const updateConfig = (updates: Partial<FailoverConfig>) => {
    setConfig((prev) => ({ ...prev, ...updates }));
    setHasChanges(true);
  };

  const getFailureTypeLabel = (type: string): string => {
    switch (type) {
      case "QuotaExceeded":
        return "配额超限";
      case "ServiceUnavailable":
        return "服务不可用";
      case "AuthenticationFailed":
        return "认证失败";
      default:
        return type;
    }
  };

  const getFailureTypeColor = (type: string): string => {
    switch (type) {
      case "QuotaExceeded":
        return "text-yellow-600 bg-yellow-50 dark:bg-yellow-950/30";
      case "ServiceUnavailable":
        return "text-red-600 bg-red-50 dark:bg-red-950/30";
      case "AuthenticationFailed":
        return "text-orange-600 bg-orange-50 dark:bg-orange-950/30";
      default:
        return "text-gray-600 bg-gray-50 dark:bg-gray-950/30";
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <HelpTip title="故障转移说明" variant="blue">
        <ul className="list-disc list-inside space-y-1 text-sm text-blue-700 dark:text-blue-400">
          <li>当 Provider 发生故障时，系统可自动切换到其他可用 Provider</li>
          <li>配额超限时自动切换可最大化利用多账号额度</li>
          <li>切换日志记录所有自动切换事件，便于追踪</li>
        </ul>
      </HelpTip>

      {error && (
        <div className="rounded-lg border border-red-500 bg-red-50 p-3 text-sm text-red-700 dark:bg-red-950/30">
          {error}
        </div>
      )}

      {success && (
        <div className="rounded-lg border border-green-500 bg-green-50 p-3 text-sm text-green-700 dark:bg-green-950/30">
          {success}
        </div>
      )}

      {/* Auto Switch Toggle */}
      <div className="space-y-4">
        <label className="flex items-center justify-between p-4 rounded-lg border cursor-pointer hover:bg-muted/50">
          <div>
            <span className="text-sm font-medium">启用自动切换</span>
            <p className="text-xs text-muted-foreground">
              当 Provider 发生故障时，自动切换到其他可用 Provider
            </p>
          </div>
          <input
            type="checkbox"
            checked={config.auto_switch}
            onChange={(e) => updateConfig({ auto_switch: e.target.checked })}
            className="w-5 h-5 rounded border-gray-300"
          />
        </label>

        <label
          className={`flex items-center justify-between p-4 rounded-lg border cursor-pointer hover:bg-muted/50 ${
            !config.auto_switch ? "opacity-50 pointer-events-none" : ""
          }`}
        >
          <div>
            <span className="text-sm font-medium">配额超限时切换</span>
            <p className="text-xs text-muted-foreground">
              当 Provider 返回配额超限错误 (429) 时，自动切换到其他 Provider
            </p>
          </div>
          <input
            type="checkbox"
            checked={config.switch_on_quota}
            onChange={(e) =>
              updateConfig({ switch_on_quota: e.target.checked })
            }
            disabled={!config.auto_switch}
            className="w-5 h-5 rounded border-gray-300"
          />
        </label>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2 pt-4 border-t">
        <button
          onClick={handleSave}
          disabled={saving || !hasChanges}
          className="flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        >
          {saving ? (
            <RefreshCw className="h-4 w-4 animate-spin" />
          ) : (
            <Save className="h-4 w-4" />
          )}
          保存
        </button>
        <button
          onClick={handleReset}
          disabled={!hasChanges}
          className="flex items-center gap-2 rounded-lg border px-4 py-2 text-sm hover:bg-muted disabled:opacity-50"
        >
          <RotateCcw className="h-4 w-4" />
          撤销更改
        </button>
      </div>

      {/* Switch Log */}
      <div className="space-y-3 pt-4 border-t">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium flex items-center gap-2">
            <ArrowRightLeft className="h-4 w-4" />
            切换日志
            {switchLog.length > 0 && (
              <span className="rounded-full bg-muted px-2 py-0.5 text-xs">
                {switchLog.length}
              </span>
            )}
          </h3>
          {switchLog.length > 0 && (
            <button
              onClick={handleClearLog}
              className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
            >
              <Trash2 className="h-3 w-3" />
              清除日志
            </button>
          )}
        </div>

        {switchLog.length === 0 ? (
          <div className="rounded-lg border border-dashed p-6 text-center">
            <p className="text-sm text-muted-foreground">暂无切换记录</p>
            <p className="text-xs text-muted-foreground mt-1">
              当发生自动切换时，记录将显示在这里
            </p>
          </div>
        ) : (
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {switchLog
              .slice()
              .reverse()
              .map((entry, index) => (
                <div
                  key={index}
                  className="flex items-center gap-3 rounded-lg border p-3 text-sm"
                >
                  <div className="flex items-center gap-2 flex-1">
                    <span className="font-medium capitalize">
                      {entry.from_provider}
                    </span>
                    <ArrowRightLeft className="h-4 w-4 text-muted-foreground" />
                    <span className="font-medium capitalize">
                      {entry.to_provider}
                    </span>
                  </div>
                  <span
                    className={`rounded-md px-2 py-0.5 text-xs ${getFailureTypeColor(
                      entry.failure_type,
                    )}`}
                  >
                    {getFailureTypeLabel(entry.failure_type)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {entry.timestamp}
                  </span>
                </div>
              ))}
          </div>
        )}
      </div>
    </div>
  );
}
