import React, { useState, useEffect } from "react";
import {
  Plus,
  MessageSquare,
  MoreHorizontal,
  Bot,
  Trash2,
  Download,
  Check,
  Loader2,
} from "lucide-react";
import styled from "styled-components";
import { skillsApi, type Skill } from "@/lib/api/skills";
import { toast } from "sonner";
import type { Topic } from "../hooks/useAgentChat";

const SidebarContainer = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: hsl(var(--muted) / 0.3);
  border-right: 1px solid hsl(var(--border));
`;

const TabsContainer = styled.div`
  display: flex;
  padding: 12px 16px 0;
  gap: 20px;
  border-bottom: 1px solid hsl(var(--border));
`;

const TabItem = styled.div<{ $active: boolean }>`
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 500;
  padding-bottom: 10px;
  cursor: pointer;
  color: ${(props) =>
    props.$active ? "hsl(var(--foreground))" : "hsl(var(--muted-foreground))"};
  border-bottom: 2px solid
    ${(props) => (props.$active ? "hsl(var(--primary))" : "transparent")};
  transition: all 0.2s;

  &:hover {
    color: hsl(var(--foreground));
  }
`;

const TabBadge = styled.span`
  font-size: 10px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  border-radius: 8px;
  background-color: hsl(var(--primary));
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
`;

const Toolbar = styled.div`
  padding: 12px 12px 8px;
`;

const NewTopicButton = styled.button`
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  color: hsl(var(--foreground));
  background-color: transparent;
  border: 1px dashed hsl(var(--border));
  transition: all 0.2s;

  &:hover {
    background-color: hsl(var(--muted));
    border-color: hsl(var(--muted-foreground));
  }
`;

const ListContainer = styled.div`
  flex: 1;
  overflow-y: auto;
  padding: 0 8px 8px;
`;

const ListItem = styled.div<{ $active: boolean }>`
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  margin-bottom: 4px;
  border-radius: 8px;
  cursor: pointer;
  background-color: ${(props) =>
    props.$active ? "hsl(var(--muted))" : "transparent"};
  color: ${(props) =>
    props.$active ? "hsl(var(--foreground))" : "hsl(var(--muted-foreground))"};
  transition: all 0.15s;

  &:hover {
    background-color: hsl(var(--muted));
    color: hsl(var(--foreground));
  }

  .title {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .delete-btn {
    opacity: 0;
    transition: opacity 0.15s;
    padding: 4px;
    border-radius: 4px;

    &:hover {
      background-color: hsl(var(--destructive) / 0.15);
      color: hsl(var(--destructive));
    }
  }

  &:hover .delete-btn {
    opacity: 1;
  }
`;

const SkillCard = styled.div<{ $installed: boolean }>`
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px;
  margin-bottom: 8px;
  border-radius: 10px;
  background-color: ${(props) =>
    props.$installed ? "hsl(var(--primary)/0.08)" : "hsl(var(--muted)/0.5)"};
  border: 1px solid
    ${(props) => (props.$installed ? "hsl(var(--primary)/0.2)" : "transparent")};
  transition: all 0.15s;

  &:hover {
    background-color: ${(props) =>
      props.$installed ? "hsl(var(--primary)/0.12)" : "hsl(var(--muted))"};
  }
`;

const SkillInfo = styled.div`
  flex: 1;
  min-width: 0;
`;

const SkillName = styled.div`
  font-size: 13px;
  font-weight: 600;
  color: hsl(var(--foreground));
  margin-bottom: 4px;
`;

const SkillDesc = styled.div`
  font-size: 11px;
  color: hsl(var(--muted-foreground));
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
`;

const SkillAction = styled.button<{ $installed: boolean }>`
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: none;
  cursor: pointer;
  transition: all 0.15s;

  ${(props) =>
    props.$installed
      ? `
    background-color: hsl(var(--primary)/0.15);
    color: hsl(var(--primary));
    &:hover {
      background-color: hsl(var(--destructive)/0.15);
      color: hsl(var(--destructive));
    }
  `
      : `
    background-color: hsl(var(--primary));
    color: white;
    &:hover {
      background-color: hsl(var(--primary)/0.9);
    }
  `}

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
`;

const SectionTitle = styled.div`
  font-size: 11px;
  font-weight: 600;
  color: hsl(var(--muted-foreground));
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 8px 12px 6px;
`;

const EmptyState = styled.div`
  text-align: center;
  padding: 24px 16px;
  color: hsl(var(--muted-foreground));
  font-size: 13px;
`;

interface ChatSidebarProps {
  onNewChat: () => void;
  topics: Topic[];
  currentTopicId: string | null;
  onSwitchTopic: (topicId: string) => void;
  onDeleteTopic: (topicId: string) => void;
  onRenameTopic?: (topicId: string, newTitle: string) => void;
}

export const ChatSidebar: React.FC<ChatSidebarProps> = ({
  onNewChat,
  topics,
  currentTopicId,
  onSwitchTopic,
  onDeleteTopic,
  onRenameTopic,
}) => {
  const [activeTab, setActiveTab] = useState<"skills" | "topics">("topics");
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loadingSkills, setLoadingSkills] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [editingTopicId, setEditingTopicId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const editInputRef = React.useRef<HTMLInputElement>(null);

  const loadSkills = async () => {
    setLoadingSkills(true);
    try {
      const allSkills = await skillsApi.getAll("proxycast");
      setSkills(allSkills);
    } catch (error) {
      console.error("加载技能列表失败:", error);
      toast.error("加载技能列表失败");
    } finally {
      setLoadingSkills(false);
    }
  };

  useEffect(() => {
    loadSkills();
  }, []);

  const handleInstall = async (skill: Skill) => {
    setActionLoading(skill.directory);
    try {
      const result = await skillsApi.install(skill.directory, "proxycast");
      if (result) {
        toast.success(`已安装: ${skill.name}`);
        await loadSkills();
      } else {
        toast.error(`安装失败: ${skill.name}`);
      }
    } catch (error: unknown) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      console.error("安装失败:", errorMsg);
      toast.error(`安装失败: ${errorMsg}`);
    } finally {
      setActionLoading(null);
    }
  };

  const handleUninstall = async (skill: Skill) => {
    setActionLoading(skill.directory);
    try {
      const result = await skillsApi.uninstall(skill.directory, "proxycast");
      if (result) {
        toast.success(`已卸载: ${skill.name}`);
        await loadSkills();
      } else {
        toast.error(`卸载失败: ${skill.name}`);
      }
    } catch (error: unknown) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      console.error("卸载失败:", errorMsg);
      toast.error(`卸载失败: ${errorMsg}`);
    } finally {
      setActionLoading(null);
    }
  };

  const handleDeleteClick = (e: React.MouseEvent, topicId: string) => {
    e.stopPropagation();
    onDeleteTopic(topicId);
  };

  // 开始编辑标题
  const handleStartEdit = (
    e: React.MouseEvent,
    topicId: string,
    currentTitle: string,
  ) => {
    e.stopPropagation();
    setEditingTopicId(topicId);
    setEditTitle(currentTitle);
  };

  // 保存编辑的标题
  const handleSaveEdit = () => {
    if (editingTopicId && editTitle.trim() && onRenameTopic) {
      onRenameTopic(editingTopicId, editTitle.trim());
    }
    setEditingTopicId(null);
    setEditTitle("");
  };

  // 取消编辑
  const handleCancelEdit = () => {
    setEditingTopicId(null);
    setEditTitle("");
  };

  // 处理输入框键盘事件
  const handleEditKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSaveEdit();
    } else if (e.key === "Escape") {
      handleCancelEdit();
    }
  };

  // 当编辑状态变化时，自动聚焦输入框
  React.useEffect(() => {
    if (editingTopicId && editInputRef.current) {
      editInputRef.current.focus();
      editInputRef.current.select();
    }
  }, [editingTopicId]);

  const installedSkills = skills.filter((s) => s.installed);
  const availableSkills = skills.filter((s) => !s.installed);

  return (
    <SidebarContainer className="w-64 shrink-0">
      <TabsContainer>
        <TabItem
          $active={activeTab === "skills"}
          onClick={() => setActiveTab("skills")}
        >
          技能
          {installedSkills.length > 0 && (
            <TabBadge>{installedSkills.length}</TabBadge>
          )}
        </TabItem>
        <TabItem
          $active={activeTab === "topics"}
          onClick={() => setActiveTab("topics")}
        >
          话题
          {topics.length > 0 && <TabBadge>{topics.length}</TabBadge>}
        </TabItem>
      </TabsContainer>

      <Toolbar>
        <NewTopicButton onClick={onNewChat}>
          <Plus size={16} />
          <span>新建话题</span>
          <MoreHorizontal size={14} className="ml-auto opacity-40" />
        </NewTopicButton>
      </Toolbar>

      <ListContainer className="custom-scrollbar">
        {activeTab === "topics" ? (
          <>
            {topics.length === 0 ? (
              <EmptyState>暂无话题，点击上方新建</EmptyState>
            ) : (
              topics.map((topic) => (
                <ListItem
                  key={topic.id}
                  $active={topic.id === currentTopicId}
                  onClick={() => {
                    if (editingTopicId !== topic.id) {
                      onSwitchTopic(topic.id);
                    }
                  }}
                  onDoubleClick={(e) =>
                    handleStartEdit(e, topic.id, topic.title)
                  }
                >
                  <MessageSquare
                    size={15}
                    className={
                      topic.id === currentTopicId
                        ? "text-primary"
                        : "opacity-50"
                    }
                  />
                  {editingTopicId === topic.id ? (
                    <input
                      ref={editInputRef}
                      type="text"
                      value={editTitle}
                      onChange={(e) => setEditTitle(e.target.value)}
                      onKeyDown={handleEditKeyDown}
                      onBlur={handleSaveEdit}
                      onClick={(e) => e.stopPropagation()}
                      style={{
                        flex: 1,
                        fontSize: "13px",
                        padding: "2px 6px",
                        border: "1px solid hsl(var(--primary))",
                        borderRadius: "4px",
                        outline: "none",
                      }}
                    />
                  ) : (
                    <span className="title">{topic.title}</span>
                  )}
                  {editingTopicId !== topic.id && (
                    <button
                      className="delete-btn"
                      onClick={(e) => handleDeleteClick(e, topic.id)}
                    >
                      <Trash2 size={14} />
                    </button>
                  )}
                </ListItem>
              ))
            )}
          </>
        ) : (
          <>
            {/* 默认助手 */}
            <ListItem $active={true}>
              <Bot size={15} className="text-primary" />
              <span className="title">默认助手</span>
            </ListItem>

            {loadingSkills ? (
              <EmptyState>
                <Loader2 size={20} className="animate-spin mx-auto mb-2" />
                加载中...
              </EmptyState>
            ) : skills.length === 0 ? (
              <EmptyState>暂无可用技能</EmptyState>
            ) : (
              <>
                {installedSkills.length > 0 && (
                  <>
                    <SectionTitle>已安装</SectionTitle>
                    {installedSkills.map((skill) => (
                      <SkillCard key={skill.directory} $installed={true}>
                        <SkillInfo>
                          <SkillName>{skill.name}</SkillName>
                          {skill.description && (
                            <SkillDesc>{skill.description}</SkillDesc>
                          )}
                        </SkillInfo>
                        <SkillAction
                          $installed={true}
                          onClick={() => handleUninstall(skill)}
                          disabled={actionLoading === skill.directory}
                          title="卸载"
                        >
                          {actionLoading === skill.directory ? (
                            <Loader2 size={14} className="animate-spin" />
                          ) : (
                            <Check size={16} />
                          )}
                        </SkillAction>
                      </SkillCard>
                    ))}
                  </>
                )}

                {availableSkills.length > 0 && (
                  <>
                    <SectionTitle>可安装</SectionTitle>
                    {availableSkills.map((skill) => (
                      <SkillCard key={skill.directory} $installed={false}>
                        <SkillInfo>
                          <SkillName>{skill.name}</SkillName>
                          {skill.description && (
                            <SkillDesc>{skill.description}</SkillDesc>
                          )}
                        </SkillInfo>
                        <SkillAction
                          $installed={false}
                          onClick={() => handleInstall(skill)}
                          disabled={actionLoading === skill.directory}
                          title="安装"
                        >
                          {actionLoading === skill.directory ? (
                            <Loader2 size={14} className="animate-spin" />
                          ) : (
                            <Download size={16} />
                          )}
                        </SkillAction>
                      </SkillCard>
                    ))}
                  </>
                )}
              </>
            )}
          </>
        )}
      </ListContainer>
    </SidebarContainer>
  );
};
