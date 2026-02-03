/**
 * @file SettingsTab.tsx
 * @description 项目设置 Tab 组件，管理项目基本设置
 * @module components/projects/tabs/SettingsTab
 * @requirements 11.1, 11.2, 11.3, 11.4, 11.5, 11.6
 */

import { useState, useEffect } from "react";
import { useProject } from "@/hooks/useProject";
import { usePersonas } from "@/hooks/usePersonas";
import { useTemplates } from "@/hooks/useTemplates";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SaveIcon, ArchiveIcon, AlertTriangleIcon } from "lucide-react";

export interface SettingsTabProps {
  /** 项目 ID */
  projectId: string;
}

/** 项目图标选项 */
const ICON_OPTIONS = [
  { value: "📝", label: "📝 笔记" },
  { value: "📚", label: "📚 书籍" },
  { value: "💡", label: "💡 创意" },
  { value: "🎯", label: "🎯 目标" },
  { value: "🚀", label: "🚀 项目" },
  { value: "🎨", label: "🎨 设计" },
  { value: "📱", label: "📱 应用" },
  { value: "🌟", label: "🌟 精选" },
];

/**
 * 项目设置 Tab 组件
 *
 * 管理项目基本信息、默认人设/模板、归档。
 */
export function SettingsTab({ projectId }: SettingsTabProps) {
  const { project, loading, update, archive } = useProject(projectId);
  const { personas } = usePersonas(projectId);
  const { templates } = useTemplates(projectId);

  const [name, setName] = useState("");
  const [icon, setIcon] = useState("📝");
  const [defaultPersonaId, setDefaultPersonaId] = useState("");
  const [defaultTemplateId, setDefaultTemplateId] = useState("");
  const [saving, setSaving] = useState(false);

  // 同步项目数据到表单
  useEffect(() => {
    if (project) {
      setName(project.name);
      setIcon(project.icon || "📝");
      setDefaultPersonaId(project.defaultPersonaId || "");
      setDefaultTemplateId(project.defaultTemplateId || "");
    }
  }, [project]);

  const handleSave = async () => {
    if (!project) return;
    setSaving(true);
    try {
      await update({
        name,
        icon,
        defaultPersonaId: defaultPersonaId || undefined,
        defaultTemplateId: defaultTemplateId || undefined,
      });
    } finally {
      setSaving(false);
    }
  };

  const handleArchive = async () => {
    if (!project) return;
    if (confirm("确认归档项目？归档后项目将从列表中隐藏。")) {
      await archive();
    }
  };

  if (loading || !project) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">加载中...</div>
      </div>
    );
  }

  const isDefault = project.isDefault;

  return (
    <div className="p-4 space-y-6 max-w-2xl">
      {/* 基本信息 */}
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-muted-foreground">基本信息</h3>

        <div className="space-y-2">
          <Label htmlFor="project-name">项目名称</Label>
          <Input
            id="project-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="输入项目名称"
            disabled={isDefault}
          />
          {isDefault && (
            <p className="text-xs text-muted-foreground">
              默认项目名称不可修改
            </p>
          )}
        </div>

        <div className="space-y-2">
          <Label>项目图标</Label>
          <Select value={icon} onValueChange={setIcon}>
            <SelectTrigger>
              <SelectValue placeholder="选择图标" />
            </SelectTrigger>
            <SelectContent>
              {ICON_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* 默认配置 */}
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-muted-foreground">默认配置</h3>

        <div className="space-y-2">
          <Label>默认人设</Label>
          <Select value={defaultPersonaId} onValueChange={setDefaultPersonaId}>
            <SelectTrigger>
              <SelectValue placeholder="选择默认人设" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="">无</SelectItem>
              {personas.map((persona) => (
                <SelectItem key={persona.id} value={persona.id}>
                  {persona.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            新建话题时自动使用的人设
          </p>
        </div>

        <div className="space-y-2">
          <Label>默认排版模板</Label>
          <Select
            value={defaultTemplateId}
            onValueChange={setDefaultTemplateId}
          >
            <SelectTrigger>
              <SelectValue placeholder="选择默认模板" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="">无</SelectItem>
              {templates.map((template) => (
                <SelectItem key={template.id} value={template.id}>
                  {template.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            新建话题时自动使用的排版模板
          </p>
        </div>
      </div>

      {/* 保存按钮 */}
      <Button onClick={handleSave} disabled={saving}>
        <SaveIcon className="h-4 w-4 mr-1" />
        {saving ? "保存中..." : "保存设置"}
      </Button>

      {/* 危险操作区域 */}
      {!isDefault && (
        <div className="space-y-4 pt-6 border-t">
          <h3 className="text-sm font-medium text-destructive flex items-center gap-2">
            <AlertTriangleIcon className="h-4 w-4" />
            危险操作
          </h3>
          <Button variant="outline" onClick={handleArchive}>
            <ArchiveIcon className="h-4 w-4 mr-1" />
            归档项目
          </Button>
        </div>
      )}

      {/* 默认项目提示 */}
      {isDefault && (
        <div className="p-4 rounded-lg bg-muted/50 text-sm text-muted-foreground">
          <p className="font-medium mb-1">💡 默认项目</p>
          <p>
            默认项目不可删除或归档。所有未分配项目的话题都会归属到默认项目。
          </p>
        </div>
      )}
    </div>
  );
}

export default SettingsTab;
