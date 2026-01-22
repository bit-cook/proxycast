/**
 * @file CanvasPanel.tsx
 * @description 画布面板组件 - 显示代码或 Markdown 预览
 * @module components/general-chat/canvas/CanvasPanel
 *
 * @requirements 3.3, 4.4
 */

import React, { useState } from "react";
import type { CanvasState } from "../types";
import { CodePreview } from "./CodePreview";
import { MarkdownPreview } from "./MarkdownPreview";

interface CanvasPanelProps {
  /** 画布状态 */
  state: CanvasState;
  /** 关闭画布回调 */
  onClose: () => void;
  /** 内容变更回调 */
  onContentChange?: (content: string) => void;
}

/**
 * 画布面板组件
 */
export const CanvasPanel: React.FC<CanvasPanelProps> = ({
  state,
  onClose,
  onContentChange,
}) => {
  const [copied, setCopied] = useState(false);

  // 复制内容
  const handleCopy = () => {
    navigator.clipboard.writeText(state.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // 下载内容
  const handleDownload = () => {
    const filename =
      state.filename ||
      (state.contentType === "code"
        ? `code.${state.language || "txt"}`
        : "content.md");
    const blob = new Blob([state.content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  };

  if (!state.isOpen) {
    return null;
  }

  return (
    <div className="flex flex-col h-full bg-background border-l border-border">
      {/* 工具栏 */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-muted/50">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-foreground">
            {state.filename ||
              (state.contentType === "code" ? "代码预览" : "内容预览")}
          </span>
          {state.language && (
            <span className="text-xs px-2 py-0.5 bg-muted rounded text-muted-foreground">
              {state.language}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          {/* 复制按钮 */}
          <button
            onClick={handleCopy}
            className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted rounded transition-colors"
            title={copied ? "已复制" : "复制"}
          >
            {copied ? (
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M5 13l4 4L19 7"
                />
              </svg>
            ) : (
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                />
              </svg>
            )}
          </button>
          {/* 下载按钮 */}
          <button
            onClick={handleDownload}
            className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted rounded transition-colors"
            title="下载"
          >
            <svg
              className="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
              />
            </svg>
          </button>
          {/* 关闭按钮 */}
          <button
            onClick={onClose}
            className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted rounded transition-colors"
            title="关闭"
          >
            <svg
              className="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      </div>

      {/* 内容区域 */}
      <div className="flex-1 overflow-auto">
        {state.contentType === "code" ? (
          <CodePreview
            code={state.content}
            language={state.language || "plaintext"}
            isEditing={state.isEditing}
            onContentChange={onContentChange}
          />
        ) : state.contentType === "markdown" ? (
          <MarkdownPreview
            content={state.content}
            isEditing={state.isEditing}
            onContentChange={onContentChange}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            无内容
          </div>
        )}
      </div>
    </div>
  );
};

export default CanvasPanel;
