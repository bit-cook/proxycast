import { useState } from "react";
import { Plus, Trash2, X, Check, Ban } from "lucide-react";
import type { ProviderType } from "@/lib/api/router";

const providerLabels: Record<ProviderType, string> = {
  kiro: "Kiro",
  gemini: "Gemini",
  qwen: "Qwen",
  antigravity: "Antigravity",
  openai: "OpenAI",
  claude: "Claude",
};

const allProviders: ProviderType[] = [
  "kiro",
  "gemini",
  "qwen",
  "antigravity",
  "openai",
  "claude",
];

interface ExclusionListProps {
  exclusions: Record<ProviderType, string[]>;
  onAdd: (provider: ProviderType, pattern: string) => Promise<void>;
  onRemove: (provider: ProviderType, pattern: string) => Promise<void>;
  loading?: boolean;
}

export function ExclusionList({
  exclusions,
  onAdd,
  onRemove,
  loading,
}: ExclusionListProps) {
  const [isAdding, setIsAdding] = useState(false);
  const [newProvider, setNewProvider] = useState<ProviderType>("gemini");
  const [newPattern, setNewPattern] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [deletingKey, setDeletingKey] = useState<string | null>(null);

  const handleAdd = async () => {
    if (!newPattern.trim()) {
      setAddError("排除模式不能为空");
      return;
    }

    // Check for duplicate
    const existingPatterns = exclusions[newProvider] || [];
    if (existingPatterns.includes(newPattern.trim())) {
      setAddError("该排除模式已存在");
      return;
    }

    try {
      await onAdd(newProvider, newPattern.trim());
      setNewPattern("");
      setIsAdding(false);
      setAddError(null);
    } catch (e) {
      setAddError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleRemove = async (provider: ProviderType, pattern: string) => {
    const key = `${provider}:${pattern}`;
    setDeletingKey(key);
    try {
      await onRemove(provider, pattern);
    } finally {
      setDeletingKey(null);
    }
  };

  const handleCancel = () => {
    setIsAdding(false);
    setNewPattern("");
    setAddError(null);
  };

  // Get all exclusions as flat list for display
  const allExclusions: { provider: ProviderType; pattern: string }[] = [];
  for (const provider of allProviders) {
    const patterns = exclusions[provider] || [];
    for (const pattern of patterns) {
      allExclusions.push({ provider, pattern });
    }
  }

  // Group by provider for display
  const groupedExclusions = allProviders
    .map((provider) => ({
      provider,
      patterns: exclusions[provider] || [],
    }))
    .filter((g) => g.patterns.length > 0);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">模型排除列表</h3>
          <p className="text-sm text-muted-foreground">
            从特定 Provider 排除某些模型，避免使用效果不好的模型
          </p>
        </div>
        {!isAdding && (
          <button
            onClick={() => setIsAdding(true)}
            disabled={loading}
            className="flex items-center gap-1 rounded-lg bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            <Plus className="h-4 w-4" />
            添加排除
          </button>
        )}
      </div>

      {/* Add new exclusion form */}
      {isAdding && (
        <div className="rounded-lg border border-primary/50 bg-primary/5 p-4 space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">
                Provider
              </label>
              <select
                value={newProvider}
                onChange={(e) => setNewProvider(e.target.value as ProviderType)}
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
              >
                {allProviders.map((p) => (
                  <option key={p} value={p}>
                    {providerLabels[p]}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">
                排除模式
              </label>
              <input
                type="text"
                value={newPattern}
                onChange={(e) => setNewPattern(e.target.value)}
                placeholder="例如: *-preview"
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
                autoFocus
              />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            支持通配符：<code className="bg-muted px-1 rounded">*-preview</code>
            、<code className="bg-muted px-1 rounded">gemini-*</code>、
            <code className="bg-muted px-1 rounded">*flash*</code>
          </p>
          {addError && <p className="text-sm text-red-500">{addError}</p>}
          <div className="flex justify-end gap-2">
            <button
              onClick={handleCancel}
              className="flex items-center gap-1 rounded-lg border px-3 py-1.5 text-sm hover:bg-muted"
            >
              <X className="h-4 w-4" />
              取消
            </button>
            <button
              onClick={handleAdd}
              className="flex items-center gap-1 rounded-lg bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90"
            >
              <Check className="h-4 w-4" />
              确认添加
            </button>
          </div>
        </div>
      )}

      {/* Exclusions list grouped by provider */}
      {groupedExclusions.length === 0 && !isAdding ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed py-8 text-muted-foreground">
          <Ban className="h-8 w-8 mb-2 opacity-50" />
          <p className="text-sm">暂无排除规则</p>
          <p className="text-xs mt-1">点击"添加排除"创建第一条规则</p>
        </div>
      ) : (
        <div className="space-y-4">
          {groupedExclusions.map(({ provider, patterns }) => (
            <div key={provider} className="rounded-lg border overflow-hidden">
              <div className="bg-muted/50 px-4 py-2 border-b">
                <h4 className="font-medium text-sm">
                  {providerLabels[provider]}
                </h4>
              </div>
              <div className="p-2 space-y-1">
                {patterns.map((pattern) => {
                  const key = `${provider}:${pattern}`;
                  return (
                    <div
                      key={key}
                      className="flex items-center justify-between rounded-lg px-3 py-2 hover:bg-muted/50 transition-colors"
                    >
                      <div className="flex items-center gap-2">
                        <Ban className="h-4 w-4 text-red-500" />
                        <span className="font-mono text-sm bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 px-2 py-0.5 rounded">
                          {pattern}
                        </span>
                      </div>
                      <button
                        onClick={() => handleRemove(provider, pattern)}
                        disabled={deletingKey === key}
                        className="rounded-lg p-1.5 text-red-500 hover:bg-red-100 dark:hover:bg-red-900/30 disabled:opacity-50 transition-colors"
                        title="删除"
                      >
                        <Trash2 className="h-4 w-4" />
                      </button>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
