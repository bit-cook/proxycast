/**
 * @file OAuth Provider 插件管理标签页
 * @description 显示和管理所有 OAuth Provider 插件
 * @module components/provider-pool/OAuthPluginTab
 */

import React, { useState, useCallback, useMemo } from "react";
import {
  Loader2,
  AlertCircle,
  RefreshCw,
  Plus,
  Download,
  Search,
  Package,
  Power,
  PowerOff,
  Trash2,
  ArrowUpCircle,
  Cloud,
  Sparkles,
  Bot,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useOAuthPlugins } from "@/hooks/useOAuthPlugins";
import { PluginUIRenderer } from "@/lib/plugin-loader/PluginUIRenderer";
import { usePluginSDK } from "@/lib/plugin-sdk";
import type {
  OAuthPluginInfo,
  PluginUpdate,
  PluginSource,
} from "@/lib/api/oauthPlugin";

// ============================================================================
// 推荐插件配置
// ============================================================================

/** 推荐插件配置 */
interface RecommendedOAuthPlugin {
  /** 插件 ID */
  id: string;
  /** 插件名称 */
  name: string;
  /** 插件描述 */
  description: string;
  /** 图标组件 */
  icon: React.ComponentType<{ className?: string }>;
  /** 目标协议 */
  targetProtocol: string;
  /** 安装来源 */
  source: PluginSource;
  /** 下载 URL（用于一键安装） */
  downloadUrl: string;
  /** 标签 */
  tags?: string[];
  /** 是否推荐 */
  recommended?: boolean;
  /** 是否可安装（false 表示即将推出） */
  available?: boolean;
}

/**
 * 推荐的 OAuth Provider 插件列表
 */
const recommendedOAuthPlugins: RecommendedOAuthPlugin[] = [
  {
    id: "kiro-provider",
    name: "Kiro (CodeWhisperer)",
    description: "Kiro (AWS CodeWhisperer) OAuth Provider - 支持 Claude 模型",
    icon: Cloud,
    targetProtocol: "anthropic",
    source: {
      type: "git_hub",
      owner: "aiclientproxy",
      repo: "kiro-provider",
      version: "v0.3.0",
    },
    downloadUrl:
      "https://github.com/aiclientproxy/kiro-provider/releases/download/v0.3.0/kiro-provider-plugin.zip",
    tags: ["anthropic", "免费"],
    recommended: true,
    available: true,
  },
  {
    id: "antigravity-provider",
    name: "Antigravity (Gemini CLI)",
    description:
      "Antigravity (Google Gemini CLI) OAuth Provider - 支持 Gemini 和 Claude 模型",
    icon: Sparkles,
    targetProtocol: "dynamic",
    source: {
      type: "git_hub",
      owner: "aiclientproxy",
      repo: "antigravity-provider",
      version: "v0.4.0",
    },
    downloadUrl:
      "https://github.com/aiclientproxy/antigravity-provider/releases/download/v0.4.0/antigravity-provider-plugin.zip",
    tags: ["gemini", "claude", "免费"],
    recommended: true,
    available: true,
  },
  {
    id: "claude-provider",
    name: "Claude Provider",
    description: "Claude OAuth Provider - 支持 Claude.ai 官方 OAuth 认证",
    icon: Bot,
    targetProtocol: "anthropic",
    source: {
      type: "git_hub",
      owner: "aiclientproxy",
      repo: "claude-provider",
      version: "v0.3.0",
    },
    downloadUrl:
      "https://github.com/aiclientproxy/claude-provider/releases/download/v0.3.0/claude-provider-plugin.zip",
    tags: ["anthropic", "官方"],
    recommended: false,
    available: true,
  },
  {
    id: "droid-provider",
    name: "Droid Provider",
    description:
      "Factory.ai Droid OAuth Provider - 支持 Anthropic 和 OpenAI 模型",
    icon: Bot,
    targetProtocol: "dynamic",
    source: {
      type: "git_hub",
      owner: "aiclientproxy",
      repo: "droid-provider",
      version: "v0.3.0",
    },
    downloadUrl:
      "https://github.com/aiclientproxy/droid-provider/releases/download/v0.3.0/droid-provider-plugin.zip",
    tags: ["anthropic", "openai"],
    recommended: false,
    available: true,
  },
  {
    id: "gemini-provider",
    name: "Gemini Provider",
    description: "Google Gemini OAuth Provider - 支持 OAuth 和 API Key 认证",
    icon: Sparkles,
    targetProtocol: "gemini",
    source: {
      type: "git_hub",
      owner: "aiclientproxy",
      repo: "gemini-provider",
      version: "v0.4.0",
    },
    downloadUrl:
      "https://github.com/aiclientproxy/gemini-provider/releases/download/v0.4.0/gemini-provider-plugin.zip",
    tags: ["gemini", "API Key"],
    recommended: false,
    available: true,
  },
  {
    id: "codex-provider",
    name: "Codex Provider",
    description: "Codex OAuth Provider - 支持 OpenAI 模型 (GPT-*, O1-*, O3-*)",
    icon: Bot,
    targetProtocol: "openai",
    source: {
      type: "git_hub",
      owner: "aiclientproxy",
      repo: "codex-provider",
      version: "v0.1.0",
    },
    downloadUrl:
      "https://github.com/aiclientproxy/codex-provider/releases/download/v0.1.0/codex-provider-plugin.zip",
    tags: ["openai", "免费"],
    recommended: false,
    available: true,
  },
];

// ============================================================================
// 推荐插件卡片组件
// ============================================================================

/**
 * 推荐插件卡片组件
 */
const RecommendedPluginCard: React.FC<{
  plugin: RecommendedOAuthPlugin;
  onInstall: () => void;
  installing?: boolean;
}> = ({ plugin, onInstall, installing }) => {
  const protocolColors: Record<string, string> = {
    anthropic: "bg-orange-500",
    openai: "bg-green-500",
    gemini: "bg-blue-500",
    qwen: "bg-purple-500",
  };

  return (
    <Card
      className={`relative transition-shadow hover:shadow-md ${!plugin.available ? "opacity-70" : ""}`}
    >
      {plugin.recommended && (
        <Badge
          className="absolute -top-2 -right-2 bg-green-500"
          variant="default"
        >
          推荐
        </Badge>
      )}
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base flex items-center gap-2">
            <div className="p-2 bg-primary/10 rounded-lg">
              <plugin.icon className="h-5 w-5 text-primary" />
            </div>
            {plugin.name}
          </CardTitle>
        </div>
        <CardDescription className="text-xs line-clamp-2">
          {plugin.description}
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-2 pb-2">
        <div className="flex items-center gap-2 flex-wrap">
          <Badge
            variant="secondary"
            className={`text-xs text-white ${
              protocolColors[plugin.targetProtocol.toLowerCase()] ||
              "bg-gray-500"
            }`}
          >
            {plugin.targetProtocol}
          </Badge>
          {plugin.tags?.map((tag) => (
            <Badge key={tag} variant="outline" className="text-xs">
              {tag}
            </Badge>
          ))}
        </div>
      </CardContent>
      <CardFooter className="pt-2">
        {plugin.available ? (
          <Button className="w-full" onClick={onInstall} disabled={installing}>
            {installing ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                安装中...
              </>
            ) : (
              <>
                <Download className="h-4 w-4 mr-2" />
                一键安装
              </>
            )}
          </Button>
        ) : (
          <Button className="w-full" variant="secondary" disabled>
            即将推出
          </Button>
        )}
      </CardFooter>
    </Card>
  );
};

/**
 * 插件卡片组件
 */
const PluginCard: React.FC<{
  plugin: OAuthPluginInfo;
  update?: PluginUpdate;
  onSelect: () => void;
  onToggle: () => void;
  onUninstall: () => void;
  onUpdate?: () => void;
}> = ({ plugin, update, onSelect, onToggle, onUninstall, onUpdate }) => {
  const protocolColors: Record<string, string> = {
    anthropic: "bg-orange-500",
    openai: "bg-green-500",
    gemini: "bg-blue-500",
    qwen: "bg-purple-500",
  };

  return (
    <Card
      className={`relative cursor-pointer transition-shadow hover:shadow-md ${
        !plugin.enabled ? "opacity-60" : ""
      }`}
      onClick={onSelect}
    >
      {update && (
        <Badge
          className="absolute -top-2 -right-2 bg-blue-500"
          variant="default"
        >
          有更新
        </Badge>
      )}
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base flex items-center gap-2">
            <Package className="h-4 w-4" />
            {plugin.displayName}
          </CardTitle>
          <Badge variant="outline" className="text-xs">
            v{plugin.version}
          </Badge>
        </div>
        <CardDescription className="text-xs line-clamp-2">
          {plugin.description || "无描述"}
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-2 pb-2">
        <div className="flex items-center gap-2 flex-wrap">
          <Badge
            variant="secondary"
            className={`text-xs text-white ${
              protocolColors[plugin.targetProtocol.toLowerCase()] ||
              "bg-gray-500"
            }`}
          >
            {plugin.targetProtocol}
          </Badge>
          <Badge variant="outline" className="text-xs">
            {plugin.credentialCount} 凭证
          </Badge>
          <Badge
            variant={plugin.enabled ? "default" : "secondary"}
            className="text-xs"
          >
            {plugin.enabled ? "已启用" : "已禁用"}
          </Badge>
        </div>
      </CardContent>
      <CardFooter className="pt-2 flex gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={(e) => {
            e.stopPropagation();
            onToggle();
          }}
        >
          {plugin.enabled ? (
            <>
              <PowerOff className="h-3 w-3 mr-1" />
              禁用
            </>
          ) : (
            <>
              <Power className="h-3 w-3 mr-1" />
              启用
            </>
          )}
        </Button>
        {update && onUpdate && (
          <Button
            variant="default"
            size="sm"
            onClick={(e) => {
              e.stopPropagation();
              onUpdate();
            }}
          >
            <ArrowUpCircle className="h-3 w-3 mr-1" />
            更新
          </Button>
        )}
        <Button
          variant="destructive"
          size="sm"
          onClick={(e) => {
            e.stopPropagation();
            onUninstall();
          }}
        >
          <Trash2 className="h-3 w-3" />
        </Button>
      </CardFooter>
    </Card>
  );
};

// ============================================================================
// 插件详情视图组件
// ============================================================================

/**
 * 插件详情视图
 * 动态加载并渲染插件自己的 UI 组件
 */
const PluginDetailView: React.FC<{
  plugin: OAuthPluginInfo;
  onBack: () => void;
  onToggle: (enabled: boolean) => void;
}> = ({ plugin, onBack, onToggle }) => {
  // 获取插件 SDK
  const { sdk } = usePluginSDK(plugin.id);

  // 插件目录路径（从配置获取）
  const pluginsDir =
    plugin.installPath?.replace(`/${plugin.id}`, "") ||
    `~/Library/Application Support/proxycast/plugins`;

  return (
    <div className="space-y-4">
      {/* 头部 */}
      <div className="flex items-center justify-between">
        <Button variant="outline" onClick={onBack}>
          返回列表
        </Button>
        <div className="flex items-center gap-2">
          <Badge variant={plugin.enabled ? "default" : "secondary"}>
            {plugin.enabled ? "已启用" : "已禁用"}
          </Badge>
          <Button
            variant="outline"
            size="sm"
            onClick={() => onToggle(!plugin.enabled)}
          >
            {plugin.enabled ? (
              <>
                <PowerOff className="h-4 w-4 mr-1" />
                禁用
              </>
            ) : (
              <>
                <Power className="h-4 w-4 mr-1" />
                启用
              </>
            )}
          </Button>
        </div>
      </div>

      {/* 插件信息卡片 */}
      <Card>
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="flex items-center gap-2">
              <Package className="h-5 w-5" />
              {plugin.displayName}
            </CardTitle>
            <Badge variant="outline">v{plugin.version}</Badge>
          </div>
          <CardDescription>{plugin.description}</CardDescription>
        </CardHeader>
      </Card>

      {/* 插件 UI - 动态加载 */}
      <Card>
        <CardContent className="pt-6">
          <PluginUIRenderer
            pluginsDir={pluginsDir}
            pluginId={plugin.id}
            uiEntry={plugin.uiEntry || "dist/index.js"}
            sdk={sdk}
            fallback={
              <div className="flex flex-col items-center justify-center p-8 text-muted-foreground">
                <Package className="h-12 w-12 mb-4 opacity-50" />
                <p className="text-center">该插件暂无 UI 界面</p>
                <p className="text-center text-sm mt-2 opacity-70">
                  请通过凭证池页面的「OAuth 凭证」标签管理此插件的凭证
                </p>
              </div>
            }
          />
        </CardContent>
      </Card>
    </div>
  );
};

/**
 * 安装插件对话框
 */
const InstallPluginDialog: React.FC<{
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onInstall: (path: string) => void;
}> = ({ open, onOpenChange, onInstall }) => {
  const [path, setPath] = useState("");

  const handleInstall = () => {
    if (path.trim()) {
      onInstall(path.trim());
      setPath("");
      onOpenChange(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>安装 OAuth Provider 插件</DialogTitle>
          <DialogDescription>
            输入本地插件目录路径或 GitHub 仓库地址
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <label className="text-sm font-medium">插件路径</label>
            <Input
              placeholder="例如: /path/to/plugin 或 owner/repo"
              value={path}
              onChange={(e) => setPath(e.target.value)}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={handleInstall} disabled={!path.trim()}>
            <Download className="h-4 w-4 mr-2" />
            安装
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

/**
 * OAuth Provider 插件管理标签页
 */
export const OAuthPluginTab: React.FC = () => {
  const {
    plugins,
    loading,
    error,
    updates,
    refresh,
    enable,
    disable,
    install,
    uninstall,
    update,
    checkUpdates,
    reload,
  } = useOAuthPlugins();

  const [searchQuery, setSearchQuery] = useState("");
  const [selectedPluginId, setSelectedPluginId] = useState<string | null>(null);
  const [installDialogOpen, setInstallDialogOpen] = useState(false);
  const [uninstallDialogOpen, setUninstallDialogOpen] = useState(false);
  const [pluginToUninstall, setPluginToUninstall] = useState<string | null>(
    null,
  );
  const [installingPluginId, setInstallingPluginId] = useState<string | null>(
    null,
  );

  // 已安装插件的 ID 集合
  const installedPluginIds = useMemo(
    () => new Set(plugins.map((p) => p.id)),
    [plugins],
  );

  // 过滤出未安装的推荐插件
  const uninstalledRecommendedPlugins = useMemo(
    () => recommendedOAuthPlugins.filter((p) => !installedPluginIds.has(p.id)),
    [installedPluginIds],
  );

  // 过滤插件
  const filteredPlugins = plugins.filter(
    (p) =>
      p.displayName.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.description?.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.targetProtocol.toLowerCase().includes(searchQuery.toLowerCase()),
  );

  // 处理安装
  const handleInstall = useCallback(
    async (path: string) => {
      const source = path.includes("/")
        ? { type: "local_file" as const, path }
        : { type: "local_file" as const, path };
      await install(source);
    },
    [install],
  );

  // 处理推荐插件安装
  const handleRecommendedInstall = useCallback(
    async (plugin: RecommendedOAuthPlugin) => {
      setInstallingPluginId(plugin.id);
      try {
        await install(plugin.source);
      } finally {
        setInstallingPluginId(null);
      }
    },
    [install],
  );

  // 处理卸载确认
  const handleUninstallConfirm = useCallback(async () => {
    if (pluginToUninstall) {
      await uninstall(pluginToUninstall);
      setPluginToUninstall(null);
      setUninstallDialogOpen(false);
      if (selectedPluginId === pluginToUninstall) {
        setSelectedPluginId(null);
      }
    }
  }, [pluginToUninstall, uninstall, selectedPluginId]);

  // 处理切换
  const handleToggle = useCallback(
    async (plugin: OAuthPluginInfo) => {
      if (plugin.enabled) {
        await disable(plugin.id);
      } else {
        await enable(plugin.id);
      }
    },
    [enable, disable],
  );

  // 获取选中的插件
  const selectedPlugin = plugins.find((p) => p.id === selectedPluginId);

  // 加载状态
  if (loading) {
    return (
      <div className="flex items-center justify-center p-16">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        <span className="ml-3 text-muted-foreground">加载插件列表...</span>
      </div>
    );
  }

  // 错误状态
  if (error) {
    return (
      <div className="flex flex-col items-center justify-center p-16">
        <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
        <p className="text-red-600 mb-4">{error}</p>
        <Button variant="outline" onClick={refresh}>
          <RefreshCw className="h-4 w-4 mr-2" />
          重试
        </Button>
      </div>
    );
  }

  // 显示插件详情
  if (selectedPlugin) {
    return (
      <PluginDetailView
        plugin={selectedPlugin}
        onBack={() => setSelectedPluginId(null)}
        onToggle={(enabled) =>
          enabled ? enable(selectedPlugin.id) : disable(selectedPlugin.id)
        }
      />
    );
  }

  return (
    <div className="space-y-4">
      {/* 工具栏 */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2 flex-1">
          <div className="relative flex-1 max-w-sm">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="搜索插件..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={checkUpdates}>
            检查更新
          </Button>
          <Button variant="outline" size="sm" onClick={reload}>
            <RefreshCw className="h-4 w-4 mr-1" />
            刷新
          </Button>
          <Button size="sm" onClick={() => setInstallDialogOpen(true)}>
            <Plus className="h-4 w-4 mr-1" />
            安装插件
          </Button>
        </div>
      </div>

      {/* 更新提示 */}
      {updates.length > 0 && (
        <Card className="border-blue-200 bg-blue-50">
          <CardContent className="py-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-blue-700">
                有 {updates.length} 个插件可更新
              </span>
              <Button variant="ghost" size="sm" className="text-blue-700">
                查看全部
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 已安装插件 - 放在上面 */}
      {filteredPlugins.length > 0 && (
        <div className="rounded-lg border bg-card">
          <div className="p-4 border-b">
            <h4 className="font-semibold flex items-center gap-2">
              <Package className="h-4 w-4" />
              已安装插件
            </h4>
            <p className="text-sm text-muted-foreground mt-1">
              点击卡片进入插件详情，管理凭证
            </p>
          </div>
          <div className="p-4">
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {filteredPlugins.map((plugin) => (
                <PluginCard
                  key={plugin.id}
                  plugin={plugin}
                  update={updates.find((u) => u.pluginId === plugin.id)}
                  onSelect={() => setSelectedPluginId(plugin.id)}
                  onToggle={() => handleToggle(plugin)}
                  onUninstall={() => {
                    setPluginToUninstall(plugin.id);
                    setUninstallDialogOpen(true);
                  }}
                  onUpdate={
                    updates.find((u) => u.pluginId === plugin.id)
                      ? () => update(plugin.id)
                      : undefined
                  }
                />
              ))}
            </div>
          </div>
        </div>
      )}

      {/* 推荐插件 - 放在下面 */}
      {uninstalledRecommendedPlugins.length > 0 && (
        <div className="rounded-lg border bg-card">
          <div className="p-4 border-b">
            <h4 className="font-semibold flex items-center gap-2">
              <Download className="h-4 w-4" />
              可安装的 OAuth Provider 插件
            </h4>
            <p className="text-sm text-muted-foreground mt-1">
              一键安装 OAuth Provider 插件，快速扩展支持的 AI 服务
            </p>
          </div>
          <div className="p-4">
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {uninstalledRecommendedPlugins.map((plugin) => (
                <RecommendedPluginCard
                  key={plugin.id}
                  plugin={plugin}
                  onInstall={() => handleRecommendedInstall(plugin)}
                  installing={installingPluginId === plugin.id}
                />
              ))}
            </div>
          </div>
        </div>
      )}

      {/* 空状态 - 没有已安装插件且没有推荐插件 */}
      {filteredPlugins.length === 0 &&
        uninstalledRecommendedPlugins.length === 0 && (
          <div className="flex flex-col items-center justify-center p-16 border rounded-lg border-dashed">
            <Package className="h-12 w-12 text-muted-foreground mb-4" />
            <p className="text-muted-foreground mb-2">
              {searchQuery ? "没有找到匹配的插件" : "暂无可用的插件"}
            </p>
            {!searchQuery && (
              <Button
                variant="outline"
                onClick={() => setInstallDialogOpen(true)}
              >
                <Plus className="h-4 w-4 mr-1" />
                手动安装插件
              </Button>
            )}
          </div>
        )}

      {/* 安装对话框 */}
      <InstallPluginDialog
        open={installDialogOpen}
        onOpenChange={setInstallDialogOpen}
        onInstall={handleInstall}
      />

      {/* 卸载确认对话框 */}
      <Dialog open={uninstallDialogOpen} onOpenChange={setUninstallDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader className="px-6 pt-6">
            <DialogTitle>确认卸载插件</DialogTitle>
            <DialogDescription>
              此操作将删除插件及其所有凭证数据，此操作无法撤销。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="px-6 pb-6 pt-4">
            <Button
              variant="outline"
              onClick={() => setUninstallDialogOpen(false)}
            >
              取消
            </Button>
            <Button variant="destructive" onClick={handleUninstallConfirm}>
              确认卸载
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

export default OAuthPluginTab;
