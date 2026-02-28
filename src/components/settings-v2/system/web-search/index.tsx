import { useEffect, useMemo, useState } from "react";
import { Globe, RefreshCw } from "lucide-react";
import { getConfig, saveConfig, type Config } from "@/hooks/useTauri";

type SearchEngine = "google" | "xiaohongshu";

export function WebSearchSettings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [draftEngine, setDraftEngine] = useState<SearchEngine>("google");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);

  const loadConfig = async () => {
    setLoading(true);
    setMessage(null);
    try {
      const nextConfig = await getConfig();
      const engine = (nextConfig.web_search?.engine || "google") as SearchEngine;
      setConfig(nextConfig);
      setDraftEngine(engine);
    } catch (error) {
      console.error("加载网络搜索配置失败:", error);
      setMessage({
        type: "error",
        text: `加载配置失败: ${error instanceof Error ? error.message : String(error)}`,
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadConfig();
  }, []);

  const currentEngine = useMemo(
    () => ((config?.web_search?.engine || "google") as SearchEngine),
    [config],
  );

  const hasUnsavedChanges = draftEngine !== currentEngine;

  const handleSave = async () => {
    if (!config || !hasUnsavedChanges) return;
    setSaving(true);
    setMessage(null);
    try {
      const nextConfig: Config = {
        ...config,
        web_search: {
          engine: draftEngine,
        },
      };
      await saveConfig(nextConfig);
      setConfig(nextConfig);
      setMessage({ type: "success", text: "网络搜索设置已保存" });
      setTimeout(() => setMessage(null), 2500);
    } catch (error) {
      setMessage({
        type: "error",
        text: `保存失败: ${error instanceof Error ? error.message : String(error)}`,
      });
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    setDraftEngine(currentEngine);
    setMessage(null);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-40">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-4 max-w-3xl pb-20">
      {message && (
        <div
          className={`rounded-lg border p-3 text-sm ${
            message.type === "error"
              ? "border-destructive bg-destructive/10 text-destructive"
              : "border-green-500 bg-green-50 text-green-700 dark:bg-green-900/20 dark:text-green-400"
          }`}
        >
          {message.text}
        </div>
      )}

      <div className="rounded-lg border p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Globe className="h-4 w-4 text-primary" />
          <div>
            <h3 className="text-sm font-medium">搜索引擎</h3>
            <p className="text-xs text-muted-foreground">
              选择用于网络搜索的默认搜索引擎。
            </p>
          </div>
        </div>

        <div className="space-y-2">
          <label htmlFor="web-search-engine" className="text-sm font-medium">
            选择搜索引擎
          </label>
          <select
            id="web-search-engine"
            value={draftEngine}
            onChange={(e) => setDraftEngine(e.target.value as SearchEngine)}
            className="w-full h-10 rounded-md border bg-background px-3 text-sm outline-none focus:ring-2 focus:ring-primary/20"
          >
            <option value="google">Google</option>
            <option value="xiaohongshu">小红书</option>
          </select>
          <p className="text-xs text-muted-foreground">
            Google 适用于通用搜索，小红书适用于中文生活方式和购物内容。
          </p>
        </div>
      </div>

      <div className="sticky bottom-0 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/80 border rounded-lg px-4 py-3 flex items-center justify-between gap-3">
        <div className="text-sm text-muted-foreground">
          {hasUnsavedChanges ? "未保存的更改" : "所有更改已保存"}
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={handleReset}
            disabled={!hasUnsavedChanges || saving}
            className="rounded-md border px-3 py-2 text-sm disabled:opacity-50"
          >
            取消
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={!hasUnsavedChanges || saving}
            className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {saving ? "保存中..." : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default WebSearchSettings;
