/**
 * @file MarkdownPreview.tsx
 * @description Markdown 预览组件 - 支持源码/预览切换
 * @module components/general-chat/canvas/MarkdownPreview
 *
 * @requirements 4.5
 */

import React, { useState } from "react";

interface MarkdownPreviewProps {
  /** Markdown 内容 */
  content: string;
  /** 是否处于编辑模式 */
  isEditing?: boolean;
  /** 内容变更回调 */
  onContentChange?: (content: string) => void;
}

/**
 * 简单的 Markdown 渲染（基础实现）
 * TODO: 后续可以集成 react-markdown 或其他库
 */
const renderMarkdown = (content: string): string => {
  let html = content
    // 转义 HTML
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    // 标题
    .replace(
      /^### (.*$)/gm,
      '<h3 class="text-lg font-semibold mt-4 mb-2">$1</h3>',
    )
    .replace(
      /^## (.*$)/gm,
      '<h2 class="text-xl font-semibold mt-4 mb-2">$1</h2>',
    )
    .replace(/^# (.*$)/gm, '<h1 class="text-2xl font-bold mt-4 mb-2">$1</h1>')
    // 粗体和斜体
    .replace(/\*\*\*(.*?)\*\*\*/g, "<strong><em>$1</em></strong>")
    .replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>")
    .replace(/\*(.*?)\*/g, "<em>$1</em>")
    // 行内代码
    .replace(
      /`([^`]+)`/g,
      '<code class="px-1 py-0.5 bg-ink-100 rounded text-sm font-mono">$1</code>',
    )
    // 链接
    .replace(
      /\[([^\]]+)\]\(([^)]+)\)/g,
      '<a href="$2" class="text-accent hover:underline" target="_blank">$1</a>',
    )
    // 列表
    .replace(/^\s*[-*]\s+(.*$)/gm, '<li class="ml-4">$1</li>')
    // 段落
    .replace(/\n\n/g, '</p><p class="mb-2">')
    // 换行
    .replace(/\n/g, "<br/>");

  return `<p class="mb-2">${html}</p>`;
};

/**
 * Markdown 预览组件
 */
export const MarkdownPreview: React.FC<MarkdownPreviewProps> = ({
  content,
  isEditing = false,
  onContentChange,
}) => {
  const [viewMode, setViewMode] = useState<"preview" | "source">("preview");

  if (isEditing) {
    return (
      <div className="h-full flex flex-col">
        {/* 模式切换 */}
        <div className="flex items-center gap-2 px-4 py-2 border-b border-ink-200 bg-ink-50">
          <button
            onClick={() => setViewMode("preview")}
            className={`px-3 py-1 text-sm rounded transition-colors ${
              viewMode === "preview"
                ? "bg-accent text-white"
                : "text-ink-600 hover:bg-ink-100"
            }`}
          >
            预览
          </button>
          <button
            onClick={() => setViewMode("source")}
            className={`px-3 py-1 text-sm rounded transition-colors ${
              viewMode === "source"
                ? "bg-accent text-white"
                : "text-ink-600 hover:bg-ink-100"
            }`}
          >
            源码
          </button>
        </div>
        {/* 内容区域 */}
        <div className="flex-1 overflow-auto">
          {viewMode === "source" ? (
            <textarea
              value={content}
              onChange={(e) => onContentChange?.(e.target.value)}
              className="w-full h-full p-4 font-mono text-sm bg-transparent resize-none outline-none"
              spellCheck={false}
            />
          ) : (
            <div
              className="p-4 prose prose-sm max-w-none"
              dangerouslySetInnerHTML={{ __html: renderMarkdown(content) }}
            />
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="h-full overflow-auto">
      <div
        className="p-4 prose prose-sm max-w-none"
        dangerouslySetInnerHTML={{ __html: renderMarkdown(content) }}
      />
    </div>
  );
};

export default MarkdownPreview;
