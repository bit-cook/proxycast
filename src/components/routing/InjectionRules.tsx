import { useState } from "react";
import {
  Plus,
  Trash2,
  ChevronUp,
  ChevronDown,
  Check,
  X,
  Power,
  PowerOff,
  Syringe,
} from "lucide-react";
import type { InjectionRule, InjectionMode } from "@/lib/api/injection";

const modeLabels: Record<InjectionMode, string> = {
  merge: "合并 (不覆盖)",
  override: "覆盖",
};

interface InjectionRulesProps {
  rules: InjectionRule[];
  enabled: boolean;
  onToggleEnabled: (enabled: boolean) => Promise<void>;
  onAdd: (rule: InjectionRule) => Promise<void>;
  onRemove: (id: string) => Promise<void>;
  onUpdate: (id: string, rule: InjectionRule) => Promise<void>;
  loading?: boolean;
}

export function InjectionRules({
  rules,
  enabled,
  onToggleEnabled,
  onAdd,
  onRemove,
  onUpdate,
  loading,
}: InjectionRulesProps) {
  const [isAdding, setIsAdding] = useState(false);
  const [newPattern, setNewPattern] = useState("");
  const [newMode, setNewMode] = useState<InjectionMode>("merge");
  const [newPriority, setNewPriority] = useState(100);
  const [newParams, setNewParams] = useState("{}");
  const [addError, setAddError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const handleAdd = async () => {
    if (!newPattern.trim()) {
      setAddError("模式不能为空");
      return;
    }

    let params: Record<string, unknown>;
    try {
      params = JSON.parse(newParams);
    } catch {
      setAddError("参数必须是有效的 JSON");
      return;
    }

    const id = `rule-${Date.now()}`;

    try {
      await onAdd({
        id,
        pattern: newPattern.trim(),
        parameters: params,
        mode: newMode,
        priority: newPriority,
        enabled: true,
      });
      setNewPattern("");
      setNewMode("merge");
      setNewPriority(100);
      setNewParams("{}");
      setIsAdding(false);
      setAddError(null);
    } catch (e) {
      setAddError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleRemove = async (id: string) => {
    setDeletingId(id);
    try {
      await onRemove(id);
    } finally {
      setDeletingId(null);
    }
  };

  const handleToggle = async (rule: InjectionRule) => {
    await onUpdate(rule.id, { ...rule, enabled: !rule.enabled });
  };

  const handlePriorityChange = async (rule: InjectionRule, delta: number) => {
    const newPriority = Math.max(1, rule.priority + delta);
    await onUpdate(rule.id, { ...rule, priority: newPriority });
  };

  const handleCancel = () => {
    setIsAdding(false);
    setNewPattern("");
    setNewMode("merge");
    setNewPriority(100);
    setNewParams("{}");
    setAddError(null);
  };

  // Sort rules by priority
  const sortedRules = [...rules].sort((a, b) => {
    const aExact = !a.pattern.includes("*");
    const bExact = !b.pattern.includes("*");
    if (aExact !== bExact) return aExact ? -1 : 1;
    return a.priority - b.priority;
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold flex items-center gap-2">
            <Syringe className="h-5 w-5" />
            参数注入
          </h3>
          <p className="text-sm text-muted-foreground">
            向匹配的请求自动注入默认参数
          </p>
        </div>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => onToggleEnabled(e.target.checked)}
              className="rounded border-gray-300"
            />
            启用注入
          </label>
          {!isAdding && enabled && (
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
      </div>

      {/* Help text */}
      <div className="rounded-lg bg-muted/50 p-3 text-xs text-muted-foreground">
        <p className="font-medium mb-1">参数注入说明：</p>
        <ul className="list-disc list-inside space-y-0.5">
          <li>
            <code className="bg-muted px-1 rounded">合并模式</code> -
            不覆盖请求中已有的参数
          </li>
          <li>
            <code className="bg-muted px-1 rounded">覆盖模式</code> -
            强制覆盖请求中的参数
          </li>
          <li>支持与路由规则相同的通配符模式</li>
          <li>参数使用 JSON 格式，如 {`{"temperature": 0.7}`}</li>
        </ul>
      </div>

      {/* Add new rule form */}
      {isAdding && enabled && (
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
                placeholder="例如: claude-* 或 *"
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
                autoFocus
              />
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">
                注入模式
              </label>
              <select
                value={newMode}
                onChange={(e) => setNewMode(e.target.value as InjectionMode)}
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
              >
                <option value="merge">{modeLabels.merge}</option>
                <option value="override">{modeLabels.override}</option>
              </select>
            </div>
            <div>
              <label className="text-xs text-muted-foreground mb-1 block">
                优先级 (数字越小越优先)
              </label>
              <input
                type="number"
                value={newPriority}
                onChange={(e) =>
                  setNewPriority(parseInt(e.target.value) || 100)
                }
                min={1}
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
              />
            </div>
          </div>
          <div>
            <label className="text-xs text-muted-foreground mb-1 block">
              注入参数 (JSON)
            </label>
            <textarea
              value={newParams}
              onChange={(e) => setNewParams(e.target.value)}
              placeholder='{"temperature": 0.7, "max_tokens": 4096}'
              rows={3}
              className="w-full rounded-md border bg-background px-3 py-2 text-sm font-mono focus:border-primary focus:outline-none"
            />
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
      {!enabled ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed py-8 text-muted-foreground">
          <p className="text-sm">参数注入已禁用</p>
          <p className="text-xs mt-1">启用后可添加注入规则</p>
        </div>
      ) : sortedRules.length === 0 && !isAdding ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed py-8 text-muted-foreground">
          <p className="text-sm">暂无注入规则</p>
          <p className="text-xs mt-1">点击"添加规则"创建第一条规则</p>
        </div>
      ) : (
        <div className="space-y-2">
          {sortedRules.map((rule) => (
            <div
              key={rule.id}
              className={`rounded-lg border p-3 transition-colors ${
                rule.enabled ? "hover:bg-muted/50" : "opacity-60 bg-muted/30"
              }`}
            >
              <div className="flex items-center justify-between">
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
                  <span
                    className={`text-xs px-2 py-0.5 rounded ${
                      rule.mode === "merge"
                        ? "bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400"
                        : "bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-400"
                    }`}
                  >
                    {modeLabels[rule.mode]}
                  </span>
                  <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded">
                    优先级: {rule.priority}
                  </span>
                </div>
                <div className="flex items-center gap-1 shrink-0">
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
                  <button
                    onClick={() => handleRemove(rule.id)}
                    disabled={deletingId === rule.id}
                    className="rounded-lg p-1.5 text-red-500 hover:bg-red-100 dark:hover:bg-red-900/30 disabled:opacity-50 transition-colors"
                    title="删除"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
              <div className="mt-2 text-xs font-mono bg-muted/50 rounded p-2 overflow-x-auto">
                {JSON.stringify(rule.parameters, null, 2)}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
