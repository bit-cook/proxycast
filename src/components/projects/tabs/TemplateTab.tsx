/**
 * @file TemplateTab.tsx
 * @description 排版模板 Tab 组件，管理项目排版模板
 * @module components/projects/tabs/TemplateTab
 * @requirements 8.1, 8.2, 8.3, 8.4, 8.5
 */

import { useState } from "react";
import { useTemplates } from "@/hooks/useTemplates";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  PlusIcon,
  LayoutTemplateIcon,
  StarIcon,
  PencilIcon,
  TrashIcon,
  EyeIcon as _EyeIcon,
} from "lucide-react";
import type {
  Template,
  Platform,
  EmojiUsage,
  CreateTemplateRequest,
} from "@/types/template";
import { TemplateDialog } from "../dialogs";

export interface TemplateTabProps {
  /** 项目 ID */
  projectId: string;
}

/** 平台显示名称映射 */
const PLATFORM_LABELS: Record<Platform, string> = {
  xiaohongshu: "小红书",
  wechat: "微信公众号",
  zhihu: "知乎",
  weibo: "微博",
  douyin: "抖音",
  markdown: "Markdown",
};

/** Emoji 使用程度显示名称 */
const EMOJI_LABELS: Record<EmojiUsage, string> = {
  heavy: "大量使用",
  moderate: "适度使用",
  minimal: "少量使用",
};

/**
 * 排版模板 Tab 组件
 *
 * 显示模板列表，支持创建、编辑、删除和设置默认模板。
 */
export function TemplateTab({ projectId }: TemplateTabProps) {
  const { templates, loading, create, update, remove, setDefault } =
    useTemplates(projectId);
  const [editingTemplate, setEditingTemplate] = useState<Template | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const handleDelete = async (id: string) => {
    if (confirm("确定要删除这个模板吗？")) {
      await remove(id);
    }
  };

  const handleSetDefault = async (id: string) => {
    await setDefault(id);
  };

  const handleOpenCreate = () => {
    setEditingTemplate(null);
    setDialogOpen(true);
  };

  const handleOpenEdit = (template: Template) => {
    setEditingTemplate(template);
    setDialogOpen(true);
  };

  const handleSave = async (data: CreateTemplateRequest) => {
    if (editingTemplate?.id) {
      await update(editingTemplate.id, data);
    } else {
      await create(data);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">加载中...</div>
      </div>
    );
  }

  return (
    <div className="p-4 space-y-4">
      {/* 头部操作栏 */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-medium">排版模板</h2>
        <Button onClick={handleOpenCreate}>
          <PlusIcon className="h-4 w-4 mr-1" />
          创建模板
        </Button>
      </div>

      {/* 模板列表 */}
      {templates.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <LayoutTemplateIcon className="h-12 w-12 mb-4 opacity-50" />
          <p className="text-lg mb-2">暂无排版模板</p>
          <p className="text-sm mb-4">创建模板来定义内容的排版风格</p>
          <Button variant="outline" onClick={handleOpenCreate}>
            <PlusIcon className="h-4 w-4 mr-1" />
            创建模板
          </Button>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {templates.map((template) => (
            <div
              key={template.id}
              className="p-4 rounded-lg border bg-card space-y-3"
            >
              {/* 头部 */}
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-2">
                  <LayoutTemplateIcon className="h-5 w-5 text-muted-foreground" />
                  <span className="font-medium">{template.name}</span>
                  {template.isDefault && (
                    <Badge variant="secondary" className="text-xs">
                      <StarIcon className="h-3 w-3 mr-1 fill-yellow-500 text-yellow-500" />
                      默认
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={() => handleOpenEdit(template)}
                  >
                    <PencilIcon className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-destructive"
                    onClick={() => handleDelete(template.id)}
                  >
                    <TrashIcon className="h-4 w-4" />
                  </Button>
                </div>
              </div>

              {/* 平台和 Emoji 设置 */}
              <div className="flex flex-wrap gap-2 text-xs">
                <Badge variant="outline">
                  {PLATFORM_LABELS[template.platform] || template.platform}
                </Badge>
                <Badge variant="outline">
                  Emoji:{" "}
                  {EMOJI_LABELS[template.emojiUsage] || template.emojiUsage}
                </Badge>
              </div>

              {/* 样式预览 */}
              <div className="text-sm text-muted-foreground space-y-1">
                {template.titleStyle && (
                  <p className="line-clamp-1">标题: {template.titleStyle}</p>
                )}
                {template.paragraphStyle && (
                  <p className="line-clamp-1">
                    段落: {template.paragraphStyle}
                  </p>
                )}
              </div>

              {/* 操作 */}
              {!template.isDefault && (
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => handleSetDefault(template.id)}
                >
                  <StarIcon className="h-4 w-4 mr-1" />
                  设为默认
                </Button>
              )}
            </div>
          ))}
        </div>
      )}

      {/* 模板编辑对话框 */}
      <TemplateDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        projectId={projectId}
        template={editingTemplate}
        onSave={handleSave}
      />
    </div>
  );
}

export default TemplateTab;
