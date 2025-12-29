/**
 * 工具卡片右键菜单组件
 *
 * 为工具箱页面的工具卡片提供右键菜单功能
 * 支持打开工具、查看详情、启用/禁用、卸载等操作
 *
 * @module components/tools/ToolCardContextMenu
 */

import React, { useState } from "react";
import { ExternalLink, Info, Power, PowerOff, Trash2 } from "lucide-react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { ConfirmDialog } from "@/components/ConfirmDialog";

/**
 * 工具卡片数据结构
 */
interface DynamicToolCard {
  id: string;
  title: string;
  description: string;
  icon: string;
  source: "builtin" | "plugin";
  pluginId?: string;
  disabled?: boolean;
  status?: string;
}

/**
 * 页面类型
 */
type Page =
  | "provider-pool"
  | "config-management"
  | "api-server"
  | "flow-monitor"
  | "agent"
  | "tools"
  | "browser-interceptor"
  | "settings"
  | "plugins"
  | `plugin:${string}`;

interface ToolCardContextMenuProps {
  /** 工具卡片数据 */
  tool: DynamicToolCard;
  /** 子元素 */
  children: React.ReactNode;
  /** 页面导航回调 */
  onNavigate: (page: Page) => void;
  /** 切换插件启用状态回调 */
  onToggleEnabled?: (pluginId: string, enabled: boolean) => void;
  /** 卸载插件回调 */
  onUninstall?: (pluginId: string) => void;
  /** 插件是否启用 */
  isEnabled?: boolean;
}

export function ToolCardContextMenu({
  tool,
  children,
  onNavigate,
  onToggleEnabled,
  onUninstall,
  isEnabled = true,
}: ToolCardContextMenuProps) {
  const [showUninstallDialog, setShowUninstallDialog] = useState(false);

  const isPlugin = tool.source === "plugin";
  const isDisabledTool = tool.disabled;

  // 打开工具
  const handleOpen = () => {
    if (isDisabledTool) return;

    if (isPlugin && tool.pluginId) {
      onNavigate(`plugin:${tool.pluginId}`);
    } else {
      onNavigate(tool.id as Page);
    }
  };

  // 查看详情（导航到插件中心）
  const handleViewDetail = () => {
    onNavigate("plugins");
  };

  // 切换启用状态
  const handleToggleEnabled = () => {
    if (tool.pluginId && onToggleEnabled) {
      onToggleEnabled(tool.pluginId, !isEnabled);
    }
  };

  // 确认卸载
  const handleConfirmUninstall = () => {
    if (tool.pluginId && onUninstall) {
      onUninstall(tool.pluginId);
    }
    setShowUninstallDialog(false);
  };

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent className="w-48">
          {/* 打开工具 */}
          <ContextMenuItem onClick={handleOpen} disabled={isDisabledTool}>
            <ExternalLink className="mr-2 h-4 w-4" />
            打开工具
            <ContextMenuShortcut>↵</ContextMenuShortcut>
          </ContextMenuItem>

          {/* 查看详情 - 仅插件显示 */}
          {isPlugin && (
            <ContextMenuItem onClick={handleViewDetail}>
              <Info className="mr-2 h-4 w-4" />
              查看详情
              <ContextMenuShortcut>I</ContextMenuShortcut>
            </ContextMenuItem>
          )}

          {/* 插件操作 */}
          {isPlugin && (
            <>
              <ContextMenuSeparator />

              {/* 启用/禁用 */}
              <ContextMenuItem onClick={handleToggleEnabled}>
                {isEnabled ? (
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

              {/* 卸载 */}
              <ContextMenuItem
                onClick={() => setShowUninstallDialog(true)}
                className="text-red-600 focus:text-red-600"
              >
                <Trash2 className="mr-2 h-4 w-4" />
                卸载插件
                <ContextMenuShortcut>⌫</ContextMenuShortcut>
              </ContextMenuItem>
            </>
          )}
        </ContextMenuContent>
      </ContextMenu>

      {/* 卸载确认对话框 */}
      <ConfirmDialog
        isOpen={showUninstallDialog}
        title="确认卸载插件"
        message={`确定要卸载插件 "${tool.title}" 吗？此操作无法撤销。`}
        confirmText="卸载"
        onConfirm={handleConfirmUninstall}
        onCancel={() => setShowUninstallDialog(false)}
      />
    </>
  );
}
