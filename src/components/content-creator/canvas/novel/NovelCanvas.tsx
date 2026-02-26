/**
 * 小说画布组件
 *
 * 用于小说项目的章节编辑
 */

import React, { memo, useCallback, useState, useEffect } from "react";
import styled from "styled-components";
import {
  X,
  Plus,
  FileText,
  CheckCircle2,
  PanelLeftClose,
  PanelLeftOpen,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { NotionEditor } from "@/components/content-creator/canvas/document/editor";
import type { NovelCanvasState, Chapter } from "./types";
import { countWords } from "./types";
import { CanvasBreadcrumbHeader } from "../shared/CanvasBreadcrumbHeader";

const Container = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  width: 100%;
  padding: 16px;
  gap: 8px;
`;

const InnerContainer = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  background: transparent;
  border-radius: 12px;
  overflow: hidden;
`;

const Header = styled.div`
  padding: 12px 16px;
  background: hsl(var(--background));
  border-bottom: 1px solid hsl(var(--border));
  border-radius: 12px 12px 0 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
`;

const Content = styled.div`
  display: flex;
  flex: 1;
  min-height: 0;
  gap: 16px;
`;

const ChapterList = styled.div`
  width: 236px;
  min-width: 236px;
  display: flex;
  flex-direction: column;
  background: hsl(var(--background));
  border-radius: 12px;
  border: 1px solid hsl(var(--border));
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
`;

const ChapterListHeader = styled.div`
  padding: 12px;
  border-bottom: 1px solid hsl(var(--border));
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: hsl(var(--background));
`;

const ChapterListBody = styled.div`
  padding: 8px;
`;

const ChapterListFooter = styled.div`
  padding: 10px 12px;
  border-top: 1px solid hsl(var(--border));
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: hsl(var(--background));
  font-size: 12px;
  color: hsl(var(--muted-foreground));
`;

const StatItem = styled.div`
  display: flex;
  justify-content: space-between;
`;

const ChapterItem = styled.div<{ $active?: boolean }>`
  padding: 10px 12px;
  margin-bottom: 8px;
  cursor: pointer;
  border: 1px solid
    ${({ $active }) =>
      $active ? "hsl(var(--primary) / 0.4)" : "hsl(var(--border))"};
  border-radius: 10px;
  background: ${({ $active }) =>
    $active ? "hsl(var(--accent) / 0.55)" : "hsl(var(--background))"};
  box-shadow: ${({ $active }) =>
    $active ? "0 2px 8px hsl(var(--primary) / 0.12)" : "none"};
  transition: all 0.18s ease;

  &:hover {
    background: hsl(var(--accent) / 0.42);
    border-color: hsl(var(--primary) / 0.28);
  }
`;

const ChapterTitle = styled.div`
  font-weight: 500;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
  color: hsl(var(--foreground));
`;

const ChapterTitleText = styled.span`
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
`;

const ChapterMeta = styled.div`
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  margin-top: 6px;
`;

const EditorArea = styled.div`
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: hsl(var(--background));
  border-radius: 12px;
  border: 1px solid hsl(var(--border));
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  position: relative;
`;

const EditorContainer = styled.div`
  flex: 1;
  padding: 8px;
  display: flex;
  flex-direction: column;
  min-height: 0;
`;

const EmptyEditorState = styled.div`
  flex: 1;
  border: 1px dashed hsl(var(--border));
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: hsl(var(--muted-foreground));
  font-size: 14px;
`;

interface NovelCanvasProps {
  state: NovelCanvasState;
  onStateChange: (state: NovelCanvasState) => void;
  onBackHome?: () => void;
  onClose: () => void;
  useExternalToolbar?: boolean;
  chapterListCollapsed?: boolean;
  onChapterListCollapsedChange?: (collapsed: boolean) => void;
  onSelectionTextChange?: (text: string) => void;
}

/**
 * 确保章节内容第一行是标题格式 (# 标题)
 */
function ensureChapterTitleInContent(title: string, content: string): string {
  const lines = content.split("\n");
  const firstLine = lines[0] || "";

  // 如果第一行已经是 H1 标题,更新它
  if (firstLine.startsWith("# ")) {
    lines[0] = `# ${title}`;
    return lines.join("\n");
  }

  // 否则在开头插入标题
  return `# ${title}\n\n${content}`;
}

/**
 * 从章节内容中提取标题 (第一行如果是 # 格式)
 */
function extractTitleFromContent(content: string): string | null {
  const lines = content.split("\n");
  const firstLine = lines[0] || "";

  if (firstLine.startsWith("# ")) {
    return firstLine.substring(2).trim();
  }

  return null;
}

/**
 * 清理章节内容,移除第一行标题
 */
function _sanitizeChapterContent(content: string): string {
  const lines = content.split("\n");
  if (lines[0]?.startsWith("# ")) {
    return lines.slice(1).join("\n").trim();
  }
  return content;
}

export const NovelCanvas: React.FC<NovelCanvasProps> = memo(
  ({
    state,
    onStateChange,
    onBackHome,
    onClose,
    useExternalToolbar = false,
    chapterListCollapsed,
    onChapterListCollapsedChange,
    onSelectionTextChange,
  }) => {
    const [internalChapterListCollapsed, setInternalChapterListCollapsed] =
      useState(false);
    const [editorKey, setEditorKey] = useState(0);
    const isChapterListCollapsed =
      chapterListCollapsed ?? internalChapterListCollapsed;
    const currentChapter = state.chapters.find(
      (c) => c.id === state.currentChapterId,
    );

    const setChapterListCollapsed = useCallback(
      (collapsed: boolean) => {
        onChapterListCollapsedChange?.(collapsed);
        if (chapterListCollapsed === undefined) {
          setInternalChapterListCollapsed(collapsed);
        }
      },
      [chapterListCollapsed, onChapterListCollapsedChange],
    );

    const handleChapterSelect = useCallback(
      (chapterId: string) => {
        onStateChange({ ...state, currentChapterId: chapterId });
        setEditorKey((prev) => prev + 1);
      },
      [state, onStateChange],
    );

    const handleAddChapter = useCallback(() => {
      const now = Date.now();
      const chapterNumber = state.chapters.length + 1;
      const title = `第${chapterNumber}章`;
      const newChapter: Chapter = {
        id: crypto.randomUUID(),
        number: chapterNumber,
        title,
        content: `# ${title}\n\n`,
        wordCount: 0,
        status: "draft",
        createdAt: now,
        updatedAt: now,
      };
      onStateChange({
        ...state,
        chapters: [...state.chapters, newChapter],
        currentChapterId: newChapter.id,
      });
      setEditorKey((prev) => prev + 1);
    }, [state, onStateChange]);

    const handleUpdateChapter = useCallback(
      (content: string) => {
        if (!currentChapter) return;

        // 从内容中提取标题
        const extractedTitle = extractTitleFromContent(content);
        const now = Date.now();

        const updatedChapters = state.chapters.map((c) =>
          c.id === currentChapter.id
            ? {
                ...c,
                title: extractedTitle || c.title,
                content,
                wordCount: countWords(content),
                updatedAt: now,
              }
            : c,
        );
        onStateChange({ ...state, chapters: updatedChapters });
      },
      [state, currentChapter, onStateChange],
    );

    const handleToggleStatus = useCallback(() => {
      if (!currentChapter) return;
      const now = Date.now();
      const updatedChapters: Chapter[] = state.chapters.map(
        (c): Chapter =>
          c.id === currentChapter.id
            ? {
                ...c,
                status: c.status === "completed" ? "draft" : "completed",
                updatedAt: now,
              }
            : c,
      );
      onStateChange({ ...state, chapters: updatedChapters });
    }, [state, currentChapter, onStateChange]);

    // 确保当前章节内容包含标题
    useEffect(() => {
      if (currentChapter) {
        const extractedTitle = extractTitleFromContent(currentChapter.content);
        if (!extractedTitle || extractedTitle !== currentChapter.title) {
          const updatedContent = ensureChapterTitleInContent(
            currentChapter.title,
            currentChapter.content,
          );
          if (updatedContent !== currentChapter.content) {
            const updatedChapters = state.chapters.map((c) =>
              c.id === currentChapter.id
                ? { ...c, content: updatedContent }
                : c,
            );
            onStateChange({ ...state, chapters: updatedChapters });
          }
        }
      }
    }, [currentChapter, state, onStateChange]);

    useEffect(() => {
      onSelectionTextChange?.("");
    }, [state.currentChapterId, onSelectionTextChange]);

    const totalWords = state.chapters.reduce((sum, c) => sum + c.wordCount, 0);
    const completedCount = state.chapters.filter(
      (c) => c.status === "completed",
    ).length;

    return (
      <Container>
        <CanvasBreadcrumbHeader label="小说" onBackHome={onBackHome} />

        <InnerContainer>
          {!useExternalToolbar && (
            <Header>
              <div style={{ display: "flex", gap: "4px" }}>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleAddChapter}
                  title="新建章节"
                >
                  <Plus className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={onClose}
                  title="关闭"
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
            </Header>
          )}
          <Content>
            {!isChapterListCollapsed && (
              <ChapterList>
                <ChapterListHeader>
                  <span className="text-sm font-medium">章节</span>
                  {!useExternalToolbar && (
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => setChapterListCollapsed(true)}
                      title="收起章节栏"
                    >
                      <PanelLeftClose className="h-4 w-4" />
                    </Button>
                  )}
                </ChapterListHeader>
                <ScrollArea className="flex-1">
                  <ChapterListBody>
                    {state.chapters.map((chapter) => (
                      <ChapterItem
                        key={chapter.id}
                        $active={chapter.id === state.currentChapterId}
                        onClick={() => handleChapterSelect(chapter.id)}
                      >
                        <ChapterTitle>
                          {chapter.status === "completed" ? (
                            <CheckCircle2 className="h-4 w-4 text-green-500" />
                          ) : (
                            <FileText className="h-4 w-4 text-muted-foreground" />
                          )}
                          <ChapterTitleText>{chapter.title}</ChapterTitleText>
                        </ChapterTitle>
                        <ChapterMeta>{chapter.wordCount} 字</ChapterMeta>
                      </ChapterItem>
                    ))}
                  </ChapterListBody>
                </ScrollArea>
                <ChapterListFooter>
                  <StatItem>
                    <span>总章节</span>
                    <span>{state.chapters.length}</span>
                  </StatItem>
                  <StatItem>
                    <span>已完成</span>
                    <span>{completedCount}</span>
                  </StatItem>
                  <StatItem>
                    <span>总字数</span>
                    <span>{totalWords.toLocaleString()}</span>
                  </StatItem>
                </ChapterListFooter>
              </ChapterList>
            )}

            <EditorArea>
              {!useExternalToolbar && isChapterListCollapsed && (
                <div className="absolute bottom-2 left-2 z-10 flex items-center gap-1 rounded-md border bg-background/90 p-1 shadow-sm">
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => setChapterListCollapsed(false)}
                    title="展开章节栏"
                  >
                    <PanelLeftOpen className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={handleAddChapter}
                  >
                    <Plus className="h-4 w-4" />
                  </Button>
                  <Button variant="ghost" size="icon" onClick={onClose}>
                    <X className="h-4 w-4" />
                  </Button>
                </div>
              )}
              {currentChapter && (
                <EditorContainer>
                  <NotionEditor
                    key={editorKey}
                    content={currentChapter.content}
                    onChange={handleUpdateChapter}
                    onSave={handleToggleStatus}
                    onCancel={() => {}}
                    onSelectionTextChange={onSelectionTextChange}
                  />
                </EditorContainer>
              )}

              {!currentChapter && (
                <EditorContainer>
                  <EmptyEditorState>
                    请先选择章节，或在左侧新建章节开始创作
                  </EmptyEditorState>
                </EditorContainer>
              )}
            </EditorArea>
          </Content>
        </InnerContainer>
      </Container>
    );
  },
);

NovelCanvas.displayName = "NovelCanvas";
