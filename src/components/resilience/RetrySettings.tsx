import { useState, useEffect } from "react";
import { RefreshCw, Save, RotateCcw, Info } from "lucide-react";
import { resilienceApi, type RetryConfig } from "@/lib/api/resilience";
import { HelpTip } from "@/components/HelpTip";

interface RetrySettingsProps {
  onSave?: () => void;
}

// Default retryable status codes
const DEFAULT_RETRYABLE_CODES = [408, 429, 500, 502, 503, 504];

export function RetrySettings({ onSave }: RetrySettingsProps) {
  const [config, setConfig] = useState<RetryConfig>({
    max_retries: 3,
    base_delay_ms: 1000,
    max_delay_ms: 30000,
    retryable_codes: DEFAULT_RETRYABLE_CODES,
  });
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [hasChanges, setHasChanges] = useState(false);
  const [originalConfig, setOriginalConfig] = useState<RetryConfig | null>(
    null,
  );

  const loadConfig = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await resilienceApi.getRetryConfig();
      setConfig(data);
      setOriginalConfig(data);
      setHasChanges(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadConfig();
  }, []);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSuccess(null);
    try {
      await resilienceApi.updateRetryConfig(config);
      setOriginalConfig(config);
      setHasChanges(false);
      setSuccess("重试配置已保存");
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

  const handleResetToDefaults = () => {
    const defaults: RetryConfig = {
      max_retries: 3,
      base_delay_ms: 1000,
      max_delay_ms: 30000,
      retryable_codes: DEFAULT_RETRYABLE_CODES,
    };
    setConfig(defaults);
    setHasChanges(true);
  };

  const updateConfig = (updates: Partial<RetryConfig>) => {
    setConfig((prev) => ({ ...prev, ...updates }));
    setHasChanges(true);
  };

  const toggleRetryableCode = (code: number) => {
    const codes = config.retryable_codes.includes(code)
      ? config.retryable_codes.filter((c) => c !== code)
      : [...config.retryable_codes, code].sort((a, b) => a - b);
    updateConfig({ retryable_codes: codes });
  };

  // Calculate preview of backoff delays
  const calculateBackoffPreview = () => {
    const delays: number[] = [];
    for (let i = 0; i < config.max_retries; i++) {
      const exponential = config.base_delay_ms * Math.pow(2, i);
      const delay = Math.min(exponential, config.max_delay_ms);
      delays.push(delay);
    }
    return delays;
  };

  const formatDuration = (ms: number) => {
    if (ms >= 60000) {
      return `${(ms / 60000).toFixed(1)} 分钟`;
    } else if (ms >= 1000) {
      return `${(ms / 1000).toFixed(1)} 秒`;
    }
    return `${ms} 毫秒`;
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const backoffPreview = calculateBackoffPreview();

  return (
    <div className="space-y-6">
      <HelpTip title="重试机制说明" variant="blue">
        <ul className="list-disc list-inside space-y-1 text-sm text-blue-700 dark:text-blue-400">
          <li>当请求失败时，系统会自动重试指定次数</li>
          <li>使用指数退避策略，每次重试等待时间翻倍</li>
          <li>可配置哪些 HTTP 状态码触发重试</li>
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

      {/* Max Retries */}
      <div className="space-y-2">
        <label className="text-sm font-medium flex items-center gap-2">
          最大重试次数
          <span className="text-xs text-muted-foreground">(1-10)</span>
        </label>
        <div className="flex items-center gap-4">
          <input
            type="range"
            min={1}
            max={10}
            value={config.max_retries}
            onChange={(e) =>
              updateConfig({ max_retries: parseInt(e.target.value) })
            }
            className="flex-1"
          />
          <span className="w-12 text-center font-mono text-lg">
            {config.max_retries}
          </span>
        </div>
      </div>

      {/* Base Delay */}
      <div className="space-y-2">
        <label className="text-sm font-medium flex items-center gap-2">
          基础延迟
          <span className="text-xs text-muted-foreground">
            (首次重试等待时间)
          </span>
        </label>
        <div className="flex items-center gap-2">
          <input
            type="number"
            min={100}
            max={10000}
            step={100}
            value={config.base_delay_ms}
            onChange={(e) =>
              updateConfig({ base_delay_ms: parseInt(e.target.value) || 1000 })
            }
            className="w-32 rounded-lg border px-3 py-2 text-sm"
          />
          <span className="text-sm text-muted-foreground">毫秒</span>
          <span className="text-sm text-muted-foreground ml-2">
            ({formatDuration(config.base_delay_ms)})
          </span>
        </div>
      </div>

      {/* Max Delay */}
      <div className="space-y-2">
        <label className="text-sm font-medium flex items-center gap-2">
          最大延迟
          <span className="text-xs text-muted-foreground">
            (单次重试最长等待时间)
          </span>
        </label>
        <div className="flex items-center gap-2">
          <input
            type="number"
            min={1000}
            max={120000}
            step={1000}
            value={config.max_delay_ms}
            onChange={(e) =>
              updateConfig({ max_delay_ms: parseInt(e.target.value) || 30000 })
            }
            className="w-32 rounded-lg border px-3 py-2 text-sm"
          />
          <span className="text-sm text-muted-foreground">毫秒</span>
          <span className="text-sm text-muted-foreground ml-2">
            ({formatDuration(config.max_delay_ms)})
          </span>
        </div>
      </div>

      {/* Backoff Preview */}
      <div className="space-y-2">
        <label className="text-sm font-medium flex items-center gap-2">
          <Info className="h-4 w-4" />
          退避时间预览
        </label>
        <div className="rounded-lg border bg-muted/30 p-3">
          <div className="flex flex-wrap gap-2">
            {backoffPreview.map((delay, index) => (
              <div
                key={index}
                className="rounded-md bg-background px-3 py-1.5 text-sm border"
              >
                <span className="text-muted-foreground">
                  第 {index + 1} 次:
                </span>{" "}
                <span className="font-mono">{formatDuration(delay)}</span>
              </div>
            ))}
          </div>
          <p className="text-xs text-muted-foreground mt-2">
            * 实际延迟会加入随机抖动以避免请求同时重试
          </p>
        </div>
      </div>

      {/* Retryable Status Codes */}
      <div className="space-y-2">
        <label className="text-sm font-medium">可重试的 HTTP 状态码</label>
        <div className="flex flex-wrap gap-2">
          {[408, 429, 500, 502, 503, 504].map((code) => (
            <button
              key={code}
              onClick={() => toggleRetryableCode(code)}
              className={`rounded-lg border px-3 py-1.5 text-sm transition-colors ${
                config.retryable_codes.includes(code)
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border hover:border-muted-foreground/50"
              }`}
            >
              {code}
              <span className="ml-1 text-xs text-muted-foreground">
                {getStatusCodeDescription(code)}
              </span>
            </button>
          ))}
        </div>
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
        <button
          onClick={handleResetToDefaults}
          className="flex items-center gap-2 rounded-lg border px-4 py-2 text-sm hover:bg-muted ml-auto"
        >
          恢复默认
        </button>
      </div>
    </div>
  );
}

function getStatusCodeDescription(code: number): string {
  switch (code) {
    case 408:
      return "超时";
    case 429:
      return "限流";
    case 500:
      return "服务器错误";
    case 502:
      return "网关错误";
    case 503:
      return "服务不可用";
    case 504:
      return "网关超时";
    default:
      return "";
  }
}
