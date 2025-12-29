/**
 * 插件项右键菜单组件
 *
 * 为插件中心已安装插件列表提供右键菜单功能
 * 支持启用/禁用、打开目录、检查更新、卸载等操作
 *
 * @module components/plugins/PluginItemContextMenu
 */

import React, { useState } from "react";
import { open } from "@tauri-apps/plugin-shell";
import { Power, PowerOff, FolderOpen, RefreshCw, Trash2 } from "lucide-react";
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

/** 安装来源 */
interface InstallSource {
  type: "local" | "url" | "github";
  path?: string;
  url?: string;
  owner?: string;
  repo?: string;
  tag?: string;
}

/** 已安装插件信息 */
interface InstalledPlugin {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string | null;
  install_path: string;
  installed_at: string;
  source: InstallSource;
  enabled: boolean;
}

interface PluginItemContextMenuProps {
  /** 插件信息 */
  plugin: InstalledPlugin;
  /** 子元素 */
  children: React.ReactNode;
  /** 切换启用状态回调 */
  onToggleEnabled: () => void;
  /** 卸载回调 */
  onUninstall: () => void;
}

export function PluginItemContextMenu({
  plugin,
  children,
  onToggleEnabled,
  onUninstall,
}: PluginItemContextMenuProps) {
  const [showUninstallDialog, setShowUninstallDialog] = useState(false);

  // 打开插件目录
  const handleOpenFolder = async () => {
    try {
      await open(plugin.install_path);
    } catch (error) {
      console.error("打开插件目录失败:", error);
      toast.error("打开插件目录失败");
    }
  };

  // 检查更新（暂未实现）
  const handleCheckUpdate = () => {
    toast.info("检查更新功能即将推出");
  };

  // 确认卸载
  const handleConfirmUninstall = () => {
    onUninstall();
    setShowUninstallDialog(false);
  };

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent className="w-48">
          {/* 启用/禁用 */}
          <ContextMenuItem onClick={onToggleEnabled}>
            {plugin.enabled ? (
              <>
                <PowerOff className="mr-2 h-4 w-4" />
                禁用插件
              </>
            ) : (
              <>
                <Power className="mr-2 h-4 w-4" />
                启用插件
              </>
            )}
            <ContextMenuShortcut>E</ContextMenuShortcut>
          </ContextMenuItem>

          {/* 打开插件目录 */}
          <ContextMenuItem onClick={handleOpenFolder}>
            <FolderOpen className="mr-2 h-4 w-4" />
            打开插件目录
            <ContextMenuShortcut>O</ContextMenuShortcut>
          </ContextMenuItem>

          {/* 检查更新 */}
          <ContextMenuItem onClick={handleCheckUpdate} disabled>
            <RefreshCw className="mr-2 h-4 w-4" />
            检查更新
            <ContextMenuShortcut className="text-muted-foreground/50">
              即将推出
            </ContextMenuShortcut>
          </ContextMenuItem>

          <ContextMenuSeparator />

          {/* 卸载 */}
          <ContextMenuItem
            onClick={() => setShowUninstallDialog(true)}
            className="text-red-600 focus:text-red-600"
          >
            <Trash2 className="mr-2 h-4 w-4" />
            卸载
            <ContextMenuShortcut>⌫</ContextMenuShortcut>
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      {/* 卸载确认对话框 */}
      <ConfirmDialog
        isOpen={showUninstallDialog}
        title="确认卸载插件"
        message={`确定要卸载插件 "${plugin.name}" 吗？此操作无法撤销。`}
        confirmText="卸载"
        onConfirm={handleConfirmUninstall}
        onCancel={() => setShowUninstallDialog(false)}
      />
    </>
  );
}
