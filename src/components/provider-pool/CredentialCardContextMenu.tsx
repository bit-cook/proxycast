/**
 * 凭证卡片右键菜单组件
 *
 * 为凭证池的凭证卡片提供右键菜单功能
 * 支持复制 ID、刷新 Token、查看详情、启用/禁用、删除等操作
 *
 * @module components/provider-pool/CredentialCardContextMenu
 */

import React, { useState } from "react";
import { Copy, RefreshCw, Info, Power, PowerOff, Trash2 } from "lucide-react";
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
import type { CredentialDisplay } from "@/lib/api/providerPool";

interface CredentialCardContextMenuProps {
  /** 凭证数据 */
  credential: CredentialDisplay;
  /** 子元素 */
  children: React.ReactNode;
  /** 刷新 Token 回调 */
  onRefreshToken?: () => void;
  /** 切换启用状态回调 */
  onToggle: () => void;
  /** 删除回调 */
  onDelete: () => void;
  /** 是否为 OAuth 凭证 */
  isOAuth?: boolean;
}

export function CredentialCardContextMenu({
  credential,
  children,
  onRefreshToken,
  onToggle,
  onDelete,
  isOAuth = false,
}: CredentialCardContextMenuProps) {
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  // 复制凭证 ID
  const handleCopyId = async () => {
    try {
      await navigator.clipboard.writeText(credential.uuid);
      toast.success("已复制凭证 ID");
    } catch (error) {
      console.error("复制失败:", error);
      toast.error("复制失败");
    }
  };

  // 刷新 Token
  const handleRefreshToken = () => {
    if (onRefreshToken) {
      onRefreshToken();
      toast.info("正在刷新 Token...");
    }
  };

  // 查看详情（展开卡片详情）
  const handleViewDetail = () => {
    // 触发卡片展开，这里通过复制 ID 并提示用户点击卡片查看
    toast.info("请点击卡片查看详细信息");
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
          {/* 复制凭证 ID */}
          <ContextMenuItem onClick={handleCopyId}>
            <Copy className="mr-2 h-4 w-4" />
            复制凭证 ID
            <ContextMenuShortcut>C</ContextMenuShortcut>
          </ContextMenuItem>

          {/* 刷新 Token - 仅 OAuth 凭证显示 */}
          {isOAuth && onRefreshToken && (
            <ContextMenuItem onClick={handleRefreshToken}>
              <RefreshCw className="mr-2 h-4 w-4" />
              刷新 Token
              <ContextMenuShortcut>R</ContextMenuShortcut>
            </ContextMenuItem>
          )}

          {/* 查看详情 */}
          <ContextMenuItem onClick={handleViewDetail}>
            <Info className="mr-2 h-4 w-4" />
            查看详情
            <ContextMenuShortcut>I</ContextMenuShortcut>
          </ContextMenuItem>

          <ContextMenuSeparator />

          {/* 启用/禁用 */}
          <ContextMenuItem onClick={onToggle}>
            {credential.is_disabled ? (
              <>
                <Power className="mr-2 h-4 w-4" />
                启用凭证
              </>
            ) : (
              <>
                <PowerOff className="mr-2 h-4 w-4" />
                禁用凭证
              </>
            )}
            <ContextMenuShortcut>E</ContextMenuShortcut>
          </ContextMenuItem>

          {/* 删除 */}
          <ContextMenuItem
            onClick={() => setShowDeleteDialog(true)}
            className="text-red-600 focus:text-red-600"
          >
            <Trash2 className="mr-2 h-4 w-4" />
            删除凭证
            <ContextMenuShortcut>⌫</ContextMenuShortcut>
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      {/* 删除确认对话框 */}
      <ConfirmDialog
        isOpen={showDeleteDialog}
        title="确认删除凭证"
        message={`确定要删除凭证 "${credential.name || credential.uuid.slice(0, 8)}" 吗？此操作无法撤销。`}
        confirmText="删除"
        onConfirm={handleConfirmDelete}
        onCancel={() => setShowDeleteDialog(false)}
      />
    </>
  );
}
