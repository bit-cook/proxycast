/**
 * @file TopicCard.tsx
 * @description 话题卡片组件，显示话题信息和项目标签
 * @module components/projects/TopicCard
 * @requirements 3.4, 3.5
 */

import { memo } from "react";
import { cn } from "@/lib/utils";
import { MessageSquareIcon, FolderIcon } from "lucide-react";

export interface TopicCardProps {
  /** 话题 ID */
  id: string;
  /** 话题标题 */
  title: string;
  /** 最后更新时间 */
  updatedAt: number;
  /** 消息数量 */
  messageCount?: number;
  /** 项目名称 */
  projectName?: string;
  /** 项目图标 */
  projectIcon?: string;
  /** 是否为默认项目 */
  isDefaultProject?: boolean;
  /** 是否选中 */
  isActive?: boolean;
  /** 点击回调 */
  onClick?: () => void;
  /** 自定义类名 */
  className?: string;
}

/**
 * 格式化时间显示
 */
function formatTime(timestamp: number): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - date.getTime();

  // 今天
  if (diff < 24 * 60 * 60 * 1000 && date.getDate() === now.getDate()) {
    return date.toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  // 昨天
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (date.getDate() === yesterday.getDate()) {
    return "昨天";
  }

  // 本周
  if (diff < 7 * 24 * 60 * 60 * 1000) {
    const days = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
    return days[date.getDay()];
  }

  // 更早
  return date.toLocaleDateString("zh-CN", {
    month: "numeric",
    day: "numeric",
  });
}

/**
 * 话题卡片组件
 *
 * 显示话题标题、更新时间、消息数量和项目标签。
 */
export const TopicCard = memo(function TopicCard({
  title,
  updatedAt,
  messageCount,
  projectName,
  projectIcon,
  isDefaultProject,
  isActive,
  onClick,
  className,
}: TopicCardProps) {
  const displayProjectName = isDefaultProject ? "默认项目" : projectName;

  return (
    <button
      onClick={onClick}
      className={cn(
        "w-full text-left p-3 rounded-lg border transition-colors",
        "hover:bg-muted/50",
        isActive ? "bg-primary/10 border-primary/30" : "bg-card border-border",
        className,
      )}
    >
      {/* 标题 */}
      <div className="font-medium text-sm text-foreground truncate mb-1">
        {title || "新话题"}
      </div>

      {/* 底部信息 */}
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        {/* 项目标签 */}
        <div className="flex items-center gap-1 truncate">
          {projectIcon ? (
            <span className="text-sm">{projectIcon}</span>
          ) : (
            <FolderIcon className="h-3 w-3" />
          )}
          <span className="truncate">{displayProjectName || "未分类"}</span>
        </div>

        {/* 时间和消息数 */}
        <div className="flex items-center gap-2 flex-shrink-0">
          {messageCount !== undefined && messageCount > 0 && (
            <div className="flex items-center gap-0.5">
              <MessageSquareIcon className="h-3 w-3" />
              <span>{messageCount}</span>
            </div>
          )}
          <span>{formatTime(updatedAt)}</span>
        </div>
      </div>
    </button>
  );
});

export default TopicCard;
