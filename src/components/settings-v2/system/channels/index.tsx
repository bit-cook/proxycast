/**
 * 渠道管理设置页面
 *
 * Telegram / Discord / 飞书 三个 Bot 渠道的内联表单配置
 */

import React, { useState, useEffect, useCallback, useMemo } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Eye,
  EyeOff,
  Plus,
  X,
  Loader2,
  Save,
  RotateCcw,
  AlertCircle,
} from "lucide-react";
import {
  getConfig,
  saveConfig,
  Config,
  ChannelsConfig,
  TelegramBotConfig,
  DiscordBotConfig,
  FeishuBotConfig,
} from "@/hooks/useTauri";
import { useConfiguredProviders } from "@/hooks/useConfiguredProviders";

// ============================================================================
// 默认值
// ============================================================================

const DEFAULT_CHANNELS: ChannelsConfig = {
  telegram: { enabled: false, bot_token: "", allowed_user_ids: [], default_model: undefined },
  discord: { enabled: false, bot_token: "", allowed_server_ids: [], default_model: undefined },
  feishu: { enabled: false, app_id: "", app_secret: "", default_model: undefined },
};

type TabKey = "telegram" | "discord" | "feishu";

// ============================================================================
// 模型选择器子组件
// ============================================================================

function DefaultModelSelect({
  value,
  onChange,
}: {
  value: string | undefined;
  onChange: (v: string | undefined) => void;
}) {
  const { providers, loading: providersLoading } = useConfiguredProviders();

  return (
    <div>
      <label className="block text-sm font-medium mb-1.5">默认模型</label>
      <select
        value={value || ""}
        onChange={(e) => onChange(e.target.value || undefined)}
        className="w-full px-3 py-2 rounded-lg border bg-background text-sm focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none"
      >
        <option value="">未指定（使用全局默认）</option>
        {providersLoading && <option disabled>加载中...</option>}
        {providers.map((p) => (
          <optgroup key={p.key} label={p.label}>
            {p.customModels?.map((m) => (
              <option key={`${p.key}/${m}`} value={`${p.key}/${m}`}>
                {m}
              </option>
            ))}
          </optgroup>
        ))}
      </select>
      <p className="text-xs text-muted-foreground mt-1">
        为此渠道指定默认使用的 AI 模型
      </p>
    </div>
  );
}

// ============================================================================
// 密码输入组件
// ============================================================================

function PasswordInput({
  label,
  value,
  onChange,
  placeholder,
  hint,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  hint?: React.ReactNode;
}) {
  const [show, setShow] = useState(false);
  return (
    <div>
      <label className="block text-sm font-medium mb-1.5">{label}</label>
      <div className="relative">
        <input
          type={show ? "text" : "password"}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="w-full px-3 py-2 pr-10 rounded-lg border bg-background text-sm font-mono focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none"
        />
        <button
          type="button"
          onClick={() => setShow(!show)}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded hover:bg-muted"
        >
          {show ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
        </button>
      </div>
      {hint && <p className="text-xs text-muted-foreground mt-1">{hint}</p>}
    </div>
  );
}

// ============================================================================
// 字符串列表输入组件
// ============================================================================

function StringListInput({
  label,
  values,
  onChange,
  placeholder,
  hint,
}: {
  label: string;
  values: string[];
  onChange: (v: string[]) => void;
  placeholder?: string;
  hint?: string;
}) {
  const [draft, setDraft] = useState("");

  const addItem = () => {
    const trimmed = draft.trim();
    if (trimmed && !values.includes(trimmed)) {
      onChange([...values, trimmed]);
      setDraft("");
    }
  };

  return (
    <div>
      <label className="block text-sm font-medium mb-1.5">{label}</label>
      <div className="flex gap-2">
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), addItem())}
          placeholder={placeholder}
          className="flex-1 px-3 py-2 rounded-lg border bg-background text-sm focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none"
        />
        <button
          type="button"
          onClick={addItem}
          className="px-3 py-2 rounded-lg border hover:bg-muted text-sm"
        >
          <Plus className="h-4 w-4" />
        </button>
      </div>
      {values.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mt-2">
          {values.map((v) => (
            <span
              key={v}
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-muted text-sm"
            >
              {v}
              <button
                type="button"
                onClick={() => onChange(values.filter((x) => x !== v))}
                className="hover:text-destructive"
              >
                <X className="h-3 w-3" />
              </button>
            </span>
          ))}
        </div>
      )}
      {hint && <p className="text-xs text-muted-foreground mt-1">{hint}</p>}
    </div>
  );
}

// ============================================================================
// Telegram 表单
// ============================================================================

function TelegramForm({
  config,
  onChange,
}: {
  config: TelegramBotConfig;
  onChange: (c: TelegramBotConfig) => void;
}) {
  return (
    <div className="space-y-4 p-4 rounded-lg border">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium">启用 Telegram Bot</h3>
          <p className="text-xs text-muted-foreground">开启后可通过 Telegram Bot 与 AI 对话</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={config.enabled}
          onClick={() => onChange({ ...config, enabled: !config.enabled })}
          className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
            config.enabled ? "bg-primary" : "bg-muted"
          }`}
        >
          <span
            className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
              config.enabled ? "translate-x-6" : "translate-x-1"
            }`}
          />
        </button>
      </div>

      <PasswordInput
        label="Bot Token"
        value={config.bot_token}
        onChange={(v) => onChange({ ...config, bot_token: v })}
        placeholder="123456:ABC-DEF..."
        hint={
          <>
            从{" "}
            <a
              href="https://t.me/BotFather"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary underline"
            >
              @BotFather
            </a>{" "}
            获取
          </>
        }
      />

      <StringListInput
        label="允许的用户 ID"
        values={config.allowed_user_ids}
        onChange={(v) => onChange({ ...config, allowed_user_ids: v })}
        placeholder="输入 Telegram User ID"
        hint="留空则允许所有用户"
      />

      <DefaultModelSelect
        value={config.default_model}
        onChange={(v) => onChange({ ...config, default_model: v })}
      />
    </div>
  );
}

// ============================================================================
// Discord 表单
// ============================================================================

function DiscordForm({
  config,
  onChange,
}: {
  config: DiscordBotConfig;
  onChange: (c: DiscordBotConfig) => void;
}) {
  return (
    <div className="space-y-4 p-4 rounded-lg border">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium">启用 Discord Bot</h3>
          <p className="text-xs text-muted-foreground">开启后可通过 Discord Bot 与 AI 对话</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={config.enabled}
          onClick={() => onChange({ ...config, enabled: !config.enabled })}
          className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
            config.enabled ? "bg-primary" : "bg-muted"
          }`}
        >
          <span
            className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
              config.enabled ? "translate-x-6" : "translate-x-1"
            }`}
          />
        </button>
      </div>

      <PasswordInput
        label="Bot Token"
        value={config.bot_token}
        onChange={(v) => onChange({ ...config, bot_token: v })}
        placeholder="MTIz..."
        hint={
          <>
            从{" "}
            <a
              href="https://discord.com/developers/applications"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary underline"
            >
              Discord Developer Portal
            </a>{" "}
            获取
          </>
        }
      />

      <StringListInput
        label="允许的服务器 ID"
        values={config.allowed_server_ids}
        onChange={(v) => onChange({ ...config, allowed_server_ids: v })}
        placeholder="输入 Discord Server ID"
        hint="留空则允许所有服务器"
      />

      <DefaultModelSelect
        value={config.default_model}
        onChange={(v) => onChange({ ...config, default_model: v })}
      />
    </div>
  );
}

// ============================================================================
// 飞书表单
// ============================================================================

function FeishuForm({
  config,
  onChange,
}: {
  config: FeishuBotConfig;
  onChange: (c: FeishuBotConfig) => void;
}) {
  return (
    <div className="space-y-4 p-4 rounded-lg border">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium">启用飞书 Bot</h3>
          <p className="text-xs text-muted-foreground">开启后可通过飞书 Bot 与 AI 对话</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={config.enabled}
          onClick={() => onChange({ ...config, enabled: !config.enabled })}
          className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
            config.enabled ? "bg-primary" : "bg-muted"
          }`}
        >
          <span
            className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
              config.enabled ? "translate-x-6" : "translate-x-1"
            }`}
          />
        </button>
      </div>

      <div>
        <label className="block text-sm font-medium mb-1.5">App ID</label>
        <input
          type="text"
          value={config.app_id}
          onChange={(e) => onChange({ ...config, app_id: e.target.value })}
          placeholder="cli_xxxx"
          className="w-full px-3 py-2 rounded-lg border bg-background text-sm font-mono focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none"
        />
      </div>

      <PasswordInput
        label="App Secret"
        value={config.app_secret}
        onChange={(v) => onChange({ ...config, app_secret: v })}
        placeholder="飞书应用的 App Secret"
      />

      <div>
        <label className="block text-sm font-medium mb-1.5">
          Verification Token <span className="text-muted-foreground font-normal">（可选）</span>
        </label>
        <input
          type="text"
          value={config.verification_token || ""}
          onChange={(e) =>
            onChange({ ...config, verification_token: e.target.value || undefined })
          }
          placeholder="事件订阅验证 Token"
          className="w-full px-3 py-2 rounded-lg border bg-background text-sm focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none"
        />
      </div>

      <PasswordInput
        label="Encrypt Key（可选）"
        value={config.encrypt_key || ""}
        onChange={(v) => onChange({ ...config, encrypt_key: v || undefined })}
        placeholder="事件加密密钥"
      />

      <DefaultModelSelect
        value={config.default_model}
        onChange={(v) => onChange({ ...config, default_model: v })}
      />
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export interface ChannelsSettingsProps {
  className?: string;
}

export function ChannelsSettings({ className }: ChannelsSettingsProps) {
  const [activeTab, setActiveTab] = useState<TabKey>("telegram");
  const [config, setConfig] = useState<Config | null>(null);
  const [channels, setChannels] = useState<ChannelsConfig>(DEFAULT_CHANNELS);
  const [initialJson, setInitialJson] = useState("");
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  const isDirty = useMemo(
    () => JSON.stringify(channels) !== initialJson,
    [channels, initialJson],
  );

  const loadConfig = useCallback(async () => {
    try {
      const c = await getConfig();
      setConfig(c);
      const ch = c.channels ?? DEFAULT_CHANNELS;
      setChannels(ch);
      setInitialJson(JSON.stringify(ch));
    } catch (e) {
      console.error("加载配置失败", e);
    }
  }, []);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    setMessage(null);
    try {
      await saveConfig({ ...config, channels });
      setInitialJson(JSON.stringify(channels));
      setMessage({ type: "success", text: "渠道配置已保存" });
      setTimeout(() => setMessage(null), 3000);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setMessage({ type: "error", text: `保存失败: ${msg}` });
    }
    setSaving(false);
  };

  const handleCancel = () => {
    if (initialJson) {
      setChannels(JSON.parse(initialJson));
    }
  };

  if (!config) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const TAB_LABELS: Record<TabKey, string> = {
    telegram: "Telegram",
    discord: "Discord",
    feishu: "飞书",
  };

  return (
    <div className={className}>
      {message && (
        <div
          className={`rounded-lg border p-3 text-sm mb-4 ${
            message.type === "error"
              ? "border-destructive bg-destructive/10 text-destructive"
              : "border-green-500 bg-green-50 text-green-700 dark:bg-green-900/20 dark:text-green-400"
          }`}
        >
          {message.text}
        </div>
      )}

      <Tabs
        value={activeTab}
        onValueChange={(v) => setActiveTab(v as TabKey)}
        className="w-full"
      >
        <TabsList className="grid w-full max-w-md grid-cols-3">
          <TabsTrigger value="telegram">Telegram</TabsTrigger>
          <TabsTrigger value="discord">Discord</TabsTrigger>
          <TabsTrigger value="feishu">飞书</TabsTrigger>
        </TabsList>

        <TabsContent value="telegram" className="mt-6">
          <TelegramForm
            config={channels.telegram}
            onChange={(tg) => setChannels({ ...channels, telegram: tg })}
          />
        </TabsContent>

        <TabsContent value="discord" className="mt-6">
          <DiscordForm
            config={channels.discord}
            onChange={(dc) => setChannels({ ...channels, discord: dc })}
          />
        </TabsContent>

        <TabsContent value="feishu" className="mt-6">
          <FeishuForm
            config={channels.feishu}
            onChange={(fs) => setChannels({ ...channels, feishu: fs })}
          />
        </TabsContent>
      </Tabs>

      {/* 底部固定栏 */}
      {isDirty && (
        <div className="sticky bottom-0 mt-6 flex items-center justify-between rounded-lg border bg-background/95 backdrop-blur p-3">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <AlertCircle className="h-4 w-4 text-yellow-500" />
            <span>未保存的更改</span>
            <span className="rounded bg-muted px-1.5 py-0.5 text-xs font-medium">
              {TAB_LABELS[activeTab]}
            </span>
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleCancel}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-sm hover:bg-muted"
            >
              <RotateCcw className="h-3.5 w-3.5" />
              取消
            </button>
            <button
              onClick={handleSave}
              disabled={saving}
              className="flex items-center gap-1.5 px-4 py-1.5 rounded-lg bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 disabled:opacity-50"
            >
              {saving ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Save className="h-3.5 w-3.5" />}
              保存
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default ChannelsSettings;