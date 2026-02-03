/**
 * @file PersonaTab.tsx
 * @description 人设 Tab 组件，管理项目人设
 * @module components/projects/tabs/PersonaTab
 * @requirements 6.1, 6.2, 6.3, 6.4, 6.5, 6.6
 */

import { useState } from "react";
import { usePersonas } from "@/hooks/usePersonas";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  PlusIcon,
  UserIcon,
  StarIcon,
  PencilIcon,
  TrashIcon,
} from "lucide-react";
import type { Persona, CreatePersonaRequest } from "@/types/persona";
import { PersonaDialog } from "../dialogs";

export interface PersonaTabProps {
  /** 项目 ID */
  projectId: string;
}

/**
 * 人设 Tab 组件
 *
 * 显示人设列表，支持创建、编辑、删除和设置默认人设。
 */
export function PersonaTab({ projectId }: PersonaTabProps) {
  const {
    personas,
    defaultPersona: _defaultPersona,
    loading,
    create,
    update,
    remove,
    setDefault,
  } = usePersonas(projectId);
  const [editingPersona, setEditingPersona] = useState<Persona | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const handleDelete = async (id: string) => {
    if (confirm("确定要删除这个人设吗？")) {
      await remove(id);
    }
  };

  const handleSetDefault = async (id: string) => {
    await setDefault(id);
  };

  const handleOpenCreate = () => {
    setEditingPersona(null);
    setDialogOpen(true);
  };

  const handleOpenEdit = (persona: Persona) => {
    setEditingPersona(persona);
    setDialogOpen(true);
  };

  const handleSave = async (data: CreatePersonaRequest) => {
    if (editingPersona?.id) {
      await update(editingPersona.id, data);
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
        <h2 className="text-lg font-medium">人设管理</h2>
        <Button onClick={handleOpenCreate}>
          <PlusIcon className="h-4 w-4 mr-1" />
          创建人设
        </Button>
      </div>

      {/* 人设列表 */}
      {personas.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <UserIcon className="h-12 w-12 mb-4 opacity-50" />
          <p className="text-lg mb-2">暂无人设</p>
          <p className="text-sm mb-4">创建人设来定义 AI 的写作风格</p>
          <Button variant="outline" onClick={handleOpenCreate}>
            <PlusIcon className="h-4 w-4 mr-1" />
            创建人设
          </Button>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {personas.map((persona) => (
            <div
              key={persona.id}
              className="p-4 rounded-lg border bg-card space-y-3"
            >
              {/* 头部 */}
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-2">
                  <UserIcon className="h-5 w-5 text-muted-foreground" />
                  <span className="font-medium">{persona.name}</span>
                  {persona.isDefault && (
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
                    onClick={() => handleOpenEdit(persona)}
                  >
                    <PencilIcon className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-destructive"
                    onClick={() => handleDelete(persona.id)}
                  >
                    <TrashIcon className="h-4 w-4" />
                  </Button>
                </div>
              </div>

              {/* 描述 */}
              {persona.description && (
                <p className="text-sm text-muted-foreground line-clamp-2">
                  {persona.description}
                </p>
              )}

              {/* 标签 */}
              <div className="flex flex-wrap gap-2 text-xs">
                <Badge variant="outline">风格: {persona.style}</Badge>
                {persona.tone && (
                  <Badge variant="outline">语气: {persona.tone}</Badge>
                )}
                {persona.targetAudience && (
                  <Badge variant="outline">
                    受众: {persona.targetAudience}
                  </Badge>
                )}
              </div>

              {/* 操作 */}
              {!persona.isDefault && (
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => handleSetDefault(persona.id)}
                >
                  <StarIcon className="h-4 w-4 mr-1" />
                  设为默认
                </Button>
              )}
            </div>
          ))}
        </div>
      )}

      {/* 人设编辑对话框 */}
      <PersonaDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        projectId={projectId}
        persona={editingPersona}
        onSave={handleSave}
      />
    </div>
  );
}

export default PersonaTab;
