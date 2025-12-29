/**
 * 配置项右键菜单组件
 *
 * 为配置管理的配置项提供右键菜单功能
 * 支持应用配置、编辑、复制、导出、删除等操作
 *
 * @module components/clients/ConfigItemContextMenu
 */

import React, { useState } from "react";
import { Play, Edit, Copy, Download, Trash2 } from "lucide-react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { toast } from "sonner";

/** 配置项数据结构 */
interface ConfigItem {
  id: string;
  name: string;
  provider?: string;
  [key: string]: unknown;
}

interface ConfigItemContextMenuProps {
  /** 配置项数据 */
  config: ConfigItem;
  /** 是否为当前激活的配置 */
  isActive: boolean;
  /** 子元素 */
  children: React.ReactNode;
  /** 应用配置回调 */
  onApply: () => void;
  /** 编辑回调 */
  onEdit: () => void;
  /** 复制回调 */
  onDuplicate?: () => void;
  /** 导出回调 */
  onExport?: () => void;
  /** 删除回调 */
  onDelete: () => void;
}

export function ConfigItemContextMenu({
  config,
  isActive,
  children,
  onApply,
  onEdit,
  onDuplicate,
  onExport,
  onDelete,
}: ConfigItemContextMenuProps) {
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  // 复制配置
  const handleDuplicate = () => {
    if (onDuplicate) {
      onDuplicate();
    } else {
      toast.info("复制功能即将推出");
    }
  };

  // 导出配置
  const handleExport = () => {
    if (onExport) {
      onExport();
    } else {
      // 默认导出行为
      const jsonStr = JSON.stringify(config, null, 2);
      const blob = new Blob([jsonStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `config-${config.name || config.id}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success("已导出配置文件");
    }
  };

  // 确认删除
  const handleConfirmDelete = () => {
    onDelete();
    setShowDeleteDialog(false);
  };

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent className="w-48">
          {/* 应用配置 */}
          <ContextMenuItem onClick={onApply} disabled={isActive}>
            <Play className="mr-2 h-4 w-4" />
            应用配置
            {isActive && (
              <ContextMenuShortcut className="text-muted-foreground/50">
                当前
              </ContextMenuShortcut>
            )}
            {!isActive && <ContextMenuShortcut>↵</ContextMenuShortcut>}
          </ContextMenuItem>

          {/* 编辑 */}
          <ContextMenuItem onClick={onEdit}>
            <Edit className="mr-2 h-4 w-4" />
            编辑
            <ContextMenuShortcut>E</ContextMenuShortcut>
          </ContextMenuItem>

          {/* 复制 */}
          <ContextMenuItem onClick={handleDuplicate}>
            <Copy className="mr-2 h-4 w-4" />
            复制
            <ContextMenuShortcut>D</ContextMenuShortcut>
          </ContextMenuItem>

          {/* 导出 */}
          <ContextMenuItem onClick={handleExport}>
            <Download className="mr-2 h-4 w-4" />
            导出
            <ContextMenuShortcut>X</ContextMenuShortcut>
          </ContextMenuItem>

          <ContextMenuSeparator />

          {/* 删除 */}
          <ContextMenuItem
            onClick={() => setShowDeleteDialog(true)}
            className="text-red-600 focus:text-red-600"
            disabled={isActive}
          >
            <Trash2 className="mr-2 h-4 w-4" />
            删除
            {isActive && (
              <ContextMenuShortcut className="text-muted-foreground/50">
                当前
              </ContextMenuShortcut>
            )}
            {!isActive && <ContextMenuShortcut>⌫</ContextMenuShortcut>}
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      {/* 删除确认对话框 */}
      <ConfirmDialog
        isOpen={showDeleteDialog}
        title="确认删除配置"
        message={`确定要删除配置 "${config.name}" 吗？此操作无法撤销。`}
        confirmText="删除"
        onConfirm={handleConfirmDelete}
        onCancel={() => setShowDeleteDialog(false)}
      />
    </>
  );
}
