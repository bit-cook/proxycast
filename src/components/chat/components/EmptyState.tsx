/**
 * @file 空状态组件
 * @description 无消息时显示的欢迎界面
 * @module components/chat/components/EmptyState
 * @requirements 4.1, 4.4
 */

import React, { memo, useState } from "react";
import styled from "styled-components";
import {
  MessageSquare,
  Sparkles,
  Code,
  Languages,
  Lightbulb,
} from "lucide-react";
import { ProjectSelector } from "@/components/projects/ProjectSelector";

const Container = styled.div`
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 32px;
  text-align: center;
`;

const IconWrapper = styled.div`
  width: 64px;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 24px;
  border-radius: 16px;
  background: hsl(var(--primary) / 0.1);
  color: hsl(var(--primary));
`;

const Title = styled.h2`
  font-size: 24px;
  font-weight: 600;
  color: hsl(var(--foreground));
  margin-bottom: 8px;
`;

const Subtitle = styled.p`
  font-size: 15px;
  color: hsl(var(--muted-foreground));
  margin-bottom: 32px;
  max-width: 400px;
`;

const SuggestionsGrid = styled.div`
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
  max-width: 600px;
  width: 100%;
`;

const SuggestionCard = styled.button`
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px;
  border: 1px solid hsl(var(--border));
  border-radius: 12px;
  background: hsl(var(--card));
  text-align: left;
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    border-color: hsl(var(--primary));
    background: hsl(var(--primary) / 0.05);
  }
`;

const SuggestionIcon = styled.div`
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: hsl(var(--muted));
  color: hsl(var(--muted-foreground));
  flex-shrink: 0;
`;

const SuggestionContent = styled.div`
  flex: 1;
  min-width: 0;
`;

const SuggestionTitle = styled.div`
  font-size: 14px;
  font-weight: 500;
  color: hsl(var(--foreground));
  margin-bottom: 4px;
`;

const SuggestionDesc = styled.div`
  font-size: 13px;
  color: hsl(var(--muted-foreground));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
`;

const ProjectSelectorWrapper = styled.div`
  margin-bottom: 24px;
  width: 100%;
  max-width: 280px;
`;

const suggestions = [
  {
    icon: Code,
    title: "代码问答",
    desc: "解释代码、调试问题、优化建议",
    prompt: "帮我解释一下这段代码的作用",
  },
  {
    icon: Lightbulb,
    title: "概念解释",
    desc: "深入浅出地解释技术概念",
    prompt: "用简单的话解释什么是 React Hooks",
  },
  {
    icon: Languages,
    title: "翻译润色",
    desc: "翻译文本、润色表达",
    prompt: "帮我把这段话翻译成英文",
  },
  {
    icon: Sparkles,
    title: "头脑风暴",
    desc: "创意想法、方案建议",
    prompt: "帮我想几个产品名字的创意",
  },
];

interface EmptyStateProps {
  /** 点击建议时的回调 */
  onSuggestionClick?: (prompt: string) => void;
  /** 当前选中的项目 ID */
  selectedProjectId?: string | null;
  /** 项目选择变化回调 */
  onProjectChange?: (projectId: string) => void;
}

/**
 * 空状态组件
 *
 * 显示欢迎信息、项目选择器和快捷建议
 */
export const EmptyState: React.FC<EmptyStateProps> = memo(
  ({ onSuggestionClick, selectedProjectId, onProjectChange }) => {
    const [localProjectId, setLocalProjectId] = useState<string | null>(
      selectedProjectId || null,
    );

    const handleProjectChange = (projectId: string) => {
      setLocalProjectId(projectId);
      onProjectChange?.(projectId);
    };

    return (
      <Container>
        <IconWrapper>
          <MessageSquare size={32} />
        </IconWrapper>

        <Title>有什么我可以帮你的？</Title>
        <Subtitle>
          我是你的 AI 助手，可以帮你解答问题、编写代码、翻译文本、头脑风暴等
        </Subtitle>

        <ProjectSelectorWrapper>
          <ProjectSelector
            value={localProjectId}
            onChange={handleProjectChange}
            placeholder="选择项目"
          />
        </ProjectSelectorWrapper>

        <SuggestionsGrid>
          {suggestions.map((item) => (
            <SuggestionCard
              key={item.title}
              onClick={() => onSuggestionClick?.(item.prompt)}
            >
              <SuggestionIcon>
                <item.icon size={18} />
              </SuggestionIcon>
              <SuggestionContent>
                <SuggestionTitle>{item.title}</SuggestionTitle>
                <SuggestionDesc>{item.desc}</SuggestionDesc>
              </SuggestionContent>
            </SuggestionCard>
          ))}
        </SuggestionsGrid>
      </Container>
    );
  },
);

EmptyState.displayName = "EmptyState";
