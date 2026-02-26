/**
 * 插件中心页面
 *
 * 独立的插件管理页面，从设置页迁移到导航栏
 * 提供插件安装、卸载、启用/禁用等功能
 *
 * @module components/plugins/PluginsPage
 */

import { CanvasBreadcrumbHeader } from "@/components/content-creator/canvas/shared/CanvasBreadcrumbHeader";
import { buildHomeAgentParams } from "@/lib/workspace/navigation";
import type { Page, PageParams } from "@/types/page";
import { PluginManager } from "./PluginManager";

interface PluginsPageProps {
  onNavigate?: (page: Page, params?: PageParams) => void;
}

export function PluginsPage({ onNavigate }: PluginsPageProps) {
  const handleBackHome = () => {
    onNavigate?.("agent", buildHomeAgentParams());
  };

  return (
    <div className="space-y-6">
      <div className="pb-1">
        <CanvasBreadcrumbHeader label="插件中心" onBackHome={handleBackHome} />
      </div>

      <div>
        <h1 className="text-3xl font-bold">插件中心</h1>
        <p className="text-muted-foreground mt-1">管理和配置 ProxyCast 插件</p>
      </div>

      <PluginManager />
    </div>
  );
}
