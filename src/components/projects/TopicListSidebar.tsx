/**
 * @file TopicListSidebar.tsx
 * @description 话题列表侧边栏组件，包含项目筛选和话题列表
 * @module components/projects/TopicListSidebar
 * @requirements 3.1, 3.2, 3.3
 */

import { useState, useMemo } from "react";
import { ProjectFilter } from "./ProjectFilter";
import { TopicCard } from "./TopicCard";
import { Button } from "@/components/ui/button";
import { PlusIcon, SearchIcon } from "lucide-react";
import { Input } from "@/components/ui/input";

export interface Topic {
  id: string;
  title: string;
  projectId: string;
  projectName?: string;
  projectIcon?: string;
  isDefaultProject?: boolean;
  updatedAt: number;
  messageCount?: number;
}

export interface TopicListSidebarProps {
  /** 话题列表 */
  topics: Topic[];
  /** 当前选中的话题 ID */
  activeTopicId?: string | null;
  /** 话题点击回调 */
  onTopicClick?: (topicId: string) => void;
  /** 新建话题回调 */
  onNewTopic?: (projectId: string | null) => void;
  /** 加载状态 */
  loading?: boolean;
  /** 自定义类名 */
  className?: string;
}

/**
 * 话题列表侧边栏组件
 *
 * 包含项目筛选下拉框和话题列表。
 */
export function TopicListSidebar({
  topics,
  activeTopicId,
  onTopicClick,
  onNewTopic,
  loading,
  className,
}: TopicListSidebarProps) {
  const [filterProjectId, setFilterProjectId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  // 筛选后的话题列表
  const filteredTopics = useMemo(() => {
    let result = topics;

    // 按项目筛选
    if (filterProjectId) {
      result = result.filter((t) => t.projectId === filterProjectId);
    }

    // 按搜索关键词筛选
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      result = result.filter((t) => t.title.toLowerCase().includes(query));
    }

    // 按更新时间排序
    return result.sort((a, b) => b.updatedAt - a.updatedAt);
  }, [topics, filterProjectId, searchQuery]);

  return (
    <div className={`flex flex-col h-full ${className || ""}`}>
      {/* 头部 */}
      <div className="p-3 border-b space-y-3">
        {/* 项目筛选 */}
        <ProjectFilter
          value={filterProjectId}
          onChange={setFilterProjectId}
          className="w-full"
        />

        {/* 搜索框 */}
        <div className="relative">
          <SearchIcon className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="搜索话题..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8 h-8"
          />
        </div>

        {/* 新建话题按钮 */}
        <Button
          variant="outline"
          size="sm"
          className="w-full"
          onClick={() => onNewTopic?.(filterProjectId)}
        >
          <PlusIcon className="h-4 w-4 mr-1" />
          新建话题
        </Button>
      </div>

      {/* 话题列表 */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1">
        {loading ? (
          <div className="text-center text-muted-foreground py-8">
            加载中...
          </div>
        ) : filteredTopics.length === 0 ? (
          <div className="text-center text-muted-foreground py-8">
            {searchQuery || filterProjectId ? "没有找到话题" : "暂无话题"}
          </div>
        ) : (
          filteredTopics.map((topic) => (
            <TopicCard
              key={topic.id}
              id={topic.id}
              title={topic.title}
              updatedAt={topic.updatedAt}
              messageCount={topic.messageCount}
              projectName={topic.projectName}
              projectIcon={topic.projectIcon}
              isDefaultProject={topic.isDefaultProject}
              isActive={topic.id === activeTopicId}
              onClick={() => onTopicClick?.(topic.id)}
            />
          ))
        )}
      </div>
    </div>
  );
}

export default TopicListSidebar;
