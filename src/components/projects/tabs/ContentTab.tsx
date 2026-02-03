/**
 * @file ContentTab.tsx
 * @description 内容 Tab 组件，显示项目话题列表
 * @module components/projects/tabs/ContentTab
 * @requirements 5.3, 5.4, 5.5
 */

import { Button } from "@/components/ui/button";
import { PlusIcon, MessageSquareIcon } from "lucide-react";

export interface ContentTabProps {
  /** 项目 ID */
  projectId: string;
  /** 新建话题回调 */
  onNewTopic?: () => void;
  /** 话题点击回调 */
  onTopicClick?: (topicId: string) => void;
}

/**
 * 内容 Tab 组件
 *
 * 显示项目下的话题列表，提供新建话题入口。
 */
export function ContentTab({
  projectId: _projectId,
  onNewTopic,
  onTopicClick,
}: ContentTabProps) {
  // TODO: 从后端获取项目话题列表
  const topics: Array<{
    id: string;
    title: string;
    updatedAt: number;
    messageCount: number;
  }> = [];

  return (
    <div className="p-4 space-y-4">
      {/* 头部操作栏 */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-medium">话题列表</h2>
        <Button onClick={onNewTopic}>
          <PlusIcon className="h-4 w-4 mr-1" />
          新建话题
        </Button>
      </div>

      {/* 话题列表 */}
      {topics.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <MessageSquareIcon className="h-12 w-12 mb-4 opacity-50" />
          <p className="text-lg mb-2">暂无话题</p>
          <p className="text-sm mb-4">点击上方按钮创建第一个话题</p>
          <Button variant="outline" onClick={onNewTopic}>
            <PlusIcon className="h-4 w-4 mr-1" />
            新建话题
          </Button>
        </div>
      ) : (
        <div className="space-y-2">
          {topics.map((topic) => (
            <button
              key={topic.id}
              onClick={() => onTopicClick?.(topic.id)}
              className="w-full text-left p-4 rounded-lg border bg-card hover:bg-muted/50 transition-colors"
            >
              <div className="font-medium mb-1">{topic.title}</div>
              <div className="flex items-center gap-4 text-sm text-muted-foreground">
                <span>{topic.messageCount} 条消息</span>
                <span>
                  {new Date(topic.updatedAt).toLocaleDateString("zh-CN")}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default ContentTab;
