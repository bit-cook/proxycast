import { useState } from "react";
import { Plus, Trash2, ArrowRight, Check, X } from "lucide-react";
import type { ModelAlias } from "@/lib/api/router";

interface ModelMappingProps {
  aliases: ModelAlias[];
  onAdd: (alias: string, actual: string) => Promise<void>;
  onRemove: (alias: string) => Promise<void>;
  loading?: boolean;
}

export function ModelMapping({
  aliases,
  onAdd,
  onRemove,
  loading,
}: ModelMappingProps) {
  const [isAdding, setIsAdding] = useState(false);
  const [newAlias, setNewAlias] = useState("");
  const [newActual, setNewActual] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [deletingAlias, setDeletingAlias] = useState<string | null>(null);

  const handleAdd = async () => {
    if (!newAlias.trim() || !newActual.trim()) {
      setAddError("别名和实际模型名都不能为空");
      return;
    }

    // Check for duplicate alias
    if (aliases.some((a) => a.alias === newAlias.trim())) {
      setAddError("该别名已存在");
      return;
    }

    try {
      await onAdd(newAlias.trim(), newActual.trim());
      setNewAlias("");
      setNewActual("");
      setIsAdding(false);
      setAddError(null);
    } catch (e) {
      setAddError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleRemove = async (alias: string) => {
    setDeletingAlias(alias);
    try {
      await onRemove(alias);
    } finally {
      setDeletingAlias(null);
    }
  };

  const handleCancel = () => {
    setIsAdding(false);
    setNewAlias("");
    setNewActual("");
    setAddError(null);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">模型别名映射</h3>
          <p className="text-sm text-muted-foreground">
            定义模型别名，使用熟悉的名称映射到实际模型
          </p>
        </div>
        {!isAdding && (
          <button
            onClick={() => setIsAdding(true)}
            disabled={loading}
            className="flex items-center gap-1 rounded-lg bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            <Plus className="h-4 w-4" />
            添加别名
          </button>
        )}
      </div>

      {/* Add new alias form */}
      {isAdding && (
        <div className="rounded-lg border border-primary/50 bg-primary/5 p-4 space-y-3">
          <div className="flex items-center gap-3">
            <div className="flex-1">
              <label className="text-xs text-muted-foreground mb-1 block">
                别名
              </label>
              <input
                type="text"
                value={newAlias}
                onChange={(e) => setNewAlias(e.target.value)}
                placeholder="例如: gpt-4"
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
                autoFocus
              />
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground mt-5" />
            <div className="flex-1">
              <label className="text-xs text-muted-foreground mb-1 block">
                实际模型
              </label>
              <input
                type="text"
                value={newActual}
                onChange={(e) => setNewActual(e.target.value)}
                placeholder="例如: claude-sonnet-4-5-20250514"
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

      {/* Aliases list */}
      {aliases.length === 0 && !isAdding ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed py-8 text-muted-foreground">
          <p className="text-sm">暂无模型别名</p>
          <p className="text-xs mt-1">点击"添加别名"创建第一个映射</p>
        </div>
      ) : (
        <div className="space-y-2">
          {aliases.map((alias) => (
            <div
              key={alias.alias}
              className="flex items-center justify-between rounded-lg border p-3 hover:bg-muted/50 transition-colors"
            >
              <div className="flex items-center gap-3 flex-1 min-w-0">
                <span className="font-mono text-sm font-medium truncate bg-muted px-2 py-1 rounded">
                  {alias.alias}
                </span>
                <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
                <span className="font-mono text-sm text-muted-foreground truncate">
                  {alias.actual}
                </span>
              </div>
              <button
                onClick={() => handleRemove(alias.alias)}
                disabled={deletingAlias === alias.alias}
                className="rounded-lg p-2 text-red-500 hover:bg-red-100 dark:hover:bg-red-900/30 disabled:opacity-50 transition-colors shrink-0"
                title="删除"
              >
                <Trash2 className="h-4 w-4" />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
