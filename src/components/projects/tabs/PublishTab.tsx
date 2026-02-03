/**
 * @file PublishTab.tsx
 * @description 发布配置 Tab 组件，管理项目发布设置
 * @module components/projects/tabs/PublishTab
 * @requirements 9.1, 9.2, 9.3
 */

import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  SendIcon,
  CheckCircleIcon,
  XCircleIcon,
  SettingsIcon,
  HistoryIcon,
} from "lucide-react";
import type { Platform } from "@/types/template";

export interface PublishTabProps {
  /** 项目 ID */
  projectId: string;
}

/** 平台配置信息 */
interface PlatformConfig {
  platform: Platform;
  name: string;
  icon: string;
  isConfigured: boolean;
  lastPublishedAt?: number;
  publishCount: number;
}

/** 平台显示名称映射 */
const _PLATFORM_LABELS: Record<Platform, string> = {
  xiaohongshu: "小红书",
  wechat: "微信公众号",
  zhihu: "知乎",
  weibo: "微博",
  douyin: "抖音",
  markdown: "Markdown",
};

/** 模拟平台配置数据 - 实际应从 hook 获取 */
const MOCK_PLATFORMS: PlatformConfig[] = [
  {
    platform: "xiaohongshu",
    name: "小红书",
    icon: "📕",
    isConfigured: false,
    publishCount: 0,
  },
  {
    platform: "wechat",
    name: "微信公众号",
    icon: "💬",
    isConfigured: false,
    publishCount: 0,
  },
  {
    platform: "zhihu",
    name: "知乎",
    icon: "📘",
    isConfigured: false,
    publishCount: 0,
  },
  {
    platform: "weibo",
    name: "微博",
    icon: "🔴",
    isConfigured: false,
    publishCount: 0,
  },
  {
    platform: "douyin",
    name: "抖音",
    icon: "🎵",
    isConfigured: false,
    publishCount: 0,
  },
];

/**
 * 发布配置 Tab 组件
 *
 * 显示平台配置状态和发布历史。
 */
export function PublishTab({ projectId: _projectId }: PublishTabProps) {
  // TODO: 使用 usePublishConfigs hook 获取实际数据
  const platforms = MOCK_PLATFORMS;
  const loading = false;

  const _formatDate = (timestamp?: number) => {
    if (!timestamp) return "从未发布";
    return new Date(timestamp).toLocaleDateString("zh-CN", {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">加载中...</div>
      </div>
    );
  }

  return (
    <div className="p-4 space-y-6">
      {/* 头部 */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-medium">发布配置</h2>
      </div>

      {/* 平台配置列表 */}
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-muted-foreground">平台账号</h3>
        <div className="grid gap-3">
          {platforms.map((config) => (
            <div
              key={config.platform}
              className="flex items-center justify-between p-4 rounded-lg border bg-card"
            >
              <div className="flex items-center gap-3">
                <span className="text-2xl">{config.icon}</span>
                <div>
                  <p className="font-medium">{config.name}</p>
                  <p className="text-sm text-muted-foreground">
                    {config.isConfigured
                      ? `已发布 ${config.publishCount} 篇`
                      : "未配置"}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                {config.isConfigured ? (
                  <Badge variant="secondary" className="gap-1">
                    <CheckCircleIcon className="h-3 w-3 text-green-500" />
                    已连接
                  </Badge>
                ) : (
                  <Badge variant="outline" className="gap-1">
                    <XCircleIcon className="h-3 w-3 text-muted-foreground" />
                    未连接
                  </Badge>
                )}
                <Button variant="ghost" size="icon" className="h-8 w-8">
                  <SettingsIcon className="h-4 w-4" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* 发布历史 */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium text-muted-foreground">
            发布历史
          </h3>
          <Button variant="ghost" size="sm">
            <HistoryIcon className="h-4 w-4 mr-1" />
            查看全部
          </Button>
        </div>

        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground border rounded-lg">
          <SendIcon className="h-10 w-10 mb-3 opacity-50" />
          <p className="text-sm">暂无发布记录</p>
          <p className="text-xs mt-1">配置平台账号后即可发布内容</p>
        </div>
      </div>

      {/* 提示信息 */}
      <div className="p-4 rounded-lg bg-muted/50 text-sm text-muted-foreground">
        <p className="font-medium mb-1">💡 提示</p>
        <p>
          发布功能正在开发中。配置平台账号后，您可以一键将内容发布到多个平台。
        </p>
      </div>
    </div>
  );
}

export default PublishTab;
