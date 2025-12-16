import { useState } from "react";
import {
  Plus,
  Trash2,
  ArrowRight,
  ChevronUp,
  ChevronDown,
  Check,
  X,
  Power,
  PowerOff,
} from "lucide-react";
import type { RoutingRule, ProviderType } from "@/lib/api/router";

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

interface RoutingRulesProps {
  rules: RoutingRule[];
  onAdd: (rule: RoutingRule) => Promise<void>;
  onRemove: (pattern: string) => Promise<void>;
  onUpdate: (pattern: string, rule: RoutingRule) => Promise<void>;
  loading?: boolean;
}

export function RoutingRules({
  rules,
  onAdd,
  onRemove,
  onUpdate,
  loading,
}: RoutingRulesProps) {
  const [isAdding, setIsAdding] = useState(false);
  const [newPattern, setNewPattern] = useState("");
  const [newProvider, setNewProvider] = useState<ProviderType>("kiro");
  const [newPriority, setNewPriority] = useState(10);
  const [addError, setAddError] = useState<string | null>(null);
  const [deletingPattern, setDeletingPattern] = useState<string | null>(null);

  const handleAdd = async () => {
    if (!newPattern.trim()) {
      setAddError("模式不能为空");
      return;
    }

    // Check for duplicate pattern
    if (rules.some((r) => r.pattern === newPattern.trim())) {
      setAddError("该模式已存在");
      return;
    }

    try {
      await onAdd({
        pattern: newPattern.trim(),
        target_provider: newProvider,
        priority: newPriority,
        enabled: true,
      });
      setNewPattern("");
      setNewProvider("kiro");
      setNewPriority(10);
      setIsAdding(false);
      setAddError(null);
    } catch (e) {
      setAddError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleRemove = async (pattern: string) => {
    setDeletingPattern(pattern);
    try {
      await onRemove(pattern);
    } finally {
      setDeletingPattern(null);
    }
  };

  const handleToggle = async (rule: RoutingRule) => {
    await onUpdate(rule.pattern, { ...rule, enabled: !rule.enabled });
  };

  const handlePriorityChange = async (rule: RoutingRule, delta: number) => {
    const newPriority = Math.max(1, rule.priority + delta);
    await onUpdate(rule.pattern, { ...rule, priority: newPriority });
  };

  const handleCancel = () => {
    setIsAdding(false);
    setNewPattern("");
    setNewProvider("kiro");
    setNewPriority(10);
    setAddError(null);
  };

  // Sort rules by priority (lower number = higher priority)
  const sortedRules = [...rules].sort((a, b) => {
    // Exact matches first
    const aExact = !a.pattern.includes("*");
    const bExact = !b.pattern.includes("*");
    if (aExact !== bExact) return aExact ? -1 : 1;
    // Then by priority
    return a.priority - b.priority;
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">路由规则</h3>
          <p className="text-sm text-muted-foreground">
            定义模型到 Provider 的路由规则，支持通配符匹配
          </p>
        </div>
        {!isAdding && (
          <button
            onClick={() => setIsAdding(true)}
            disabled={loading}
            className="flex items-center gap-1 rounded-lg bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            <Plus className="h-4 w-4" />
            添加规则
          </button>
        )}
      </div>

      {/* Pattern help */}
      <div className="rounded-lg bg-muted/50 p-3 text-xs text-muted-foreground">
        <p className="font-medium mb-1">通配符模式说明：</p>
        <ul className="list-disc list-inside space-y-0.5">
          <li>
            <code className="bg-muted px-1 rounded">claude-*</code> - 前缀匹配
          </li>
          <li>
            <code className="bg-muted px-1 rounded">*-preview</code> - 后缀匹配
          </li>
          <li>
            <code className="bg-muted px-1 rounded">*flash*</code> - 包含匹配
          </li>
          <li>精确匹配优先于通配符匹配</li>
        </ul>
      </div>

      {/* Add new rule form */}
      {isAdding && (
        <div className="rounded-lg border border-primary/50 bg-primary/5 p-4 space-y-3">
          <div className="grid grid-cols-3 gap-3">
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">
                模式
              </label>
              <input
                type="text"
                value={newPattern}
                onChange={(e) => setNewPattern(e.target.value)}
                placeholder="例如: claude-*"
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
                autoFocus
              />
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">
                目标 Provider
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
                优先级 (数字越小越优先)
              </label>
              <input
                type="number"
                value={newPriority}
                onChange={(e) => setNewPriority(parseInt(e.target.value) || 10)}
                min={1}
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
              />
            </div>
          </div>
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

      {/* Rules list */}
      {sortedRules.length === 0 && !isAdding ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed py-8 text-muted-foreground">
          <p className="text-sm">暂无路由规则</p>
          <p className="text-xs mt-1">点击"添加规则"创建第一条规则</p>
        </div>
      ) : (
        <div className="space-y-2">
          {sortedRules.map((rule) => (
            <div
              key={rule.pattern}
              className={`flex items-center justify-between rounded-lg border p-3 transition-colors ${
                rule.enabled ? "hover:bg-muted/50" : "opacity-60 bg-muted/30"
              }`}
            >
              <div className="flex items-center gap-3 flex-1 min-w-0">
                <span
                  className={`font-mono text-sm font-medium truncate px-2 py-1 rounded ${
                    rule.pattern.includes("*")
                      ? "bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400"
                      : "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400"
                  }`}
                >
                  {rule.pattern}
                </span>
                <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
                <span className="text-sm font-medium">
                  {providerLabels[rule.target_provider]}
                </span>
                <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded">
                  优先级: {rule.priority}
                </span>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                {/* Priority adjustment */}
                <button
                  onClick={() => handlePriorityChange(rule, -1)}
                  className="rounded-lg p-1.5 text-muted-foreground hover:bg-muted transition-colors"
                  title="提高优先级"
                >
                  <ChevronUp className="h-4 w-4" />
                </button>
                <button
                  onClick={() => handlePriorityChange(rule, 1)}
                  className="rounded-lg p-1.5 text-muted-foreground hover:bg-muted transition-colors"
                  title="降低优先级"
                >
                  <ChevronDown className="h-4 w-4" />
                </button>
                {/* Toggle */}
                <button
                  onClick={() => handleToggle(rule)}
                  className={`rounded-lg p-1.5 transition-colors ${
                    rule.enabled
                      ? "text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-800"
                      : "text-green-500 hover:bg-green-100 dark:hover:bg-green-900/30"
                  }`}
                  title={rule.enabled ? "禁用" : "启用"}
                >
                  {rule.enabled ? (
                    <PowerOff className="h-4 w-4" />
                  ) : (
                    <Power className="h-4 w-4" />
                  )}
                </button>
                {/* Delete */}
                <button
                  onClick={() => handleRemove(rule.pattern)}
                  disabled={deletingPattern === rule.pattern}
                  className="rounded-lg p-1.5 text-red-500 hover:bg-red-100 dark:hover:bg-red-900/30 disabled:opacity-50 transition-colors"
                  title="删除"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
