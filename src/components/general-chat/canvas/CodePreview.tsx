/**
 * @file CodePreview.tsx
 * @description 代码预览组件 - 显示代码，支持行号和编辑
 * @module components/general-chat/canvas/CodePreview
 *
 * @requirements 4.1, 4.4
 */

import React, { useMemo } from "react";

interface CodePreviewProps {
  /** 代码内容 */
  code: string;
  /** 代码语言 */
  language: string;
  /** 是否处于编辑模式 */
  isEditing?: boolean;
  /** 内容变更回调 */
  onContentChange?: (content: string) => void;
}

/**
 * 代码预览组件
 */
export const CodePreview: React.FC<CodePreviewProps> = ({
  code,
  language: _language,
  isEditing = false,
  onContentChange,
}) => {
  // 计算行号
  const lines = useMemo(() => code.split("\n"), [code]);
  const lineCount = lines.length;
  const lineNumberWidth = String(lineCount).length * 10 + 20;

  if (isEditing) {
    return (
      <div className="h-full flex">
        {/* 行号 */}
        <div
          className="flex-shrink-0 bg-ink-50 border-r border-ink-200 text-right py-3 select-none"
          style={{ width: lineNumberWidth }}
        >
          {lines.map((_, index) => (
            <div
              key={index}
              className="px-2 text-xs text-ink-400 leading-6 font-mono"
            >
              {index + 1}
            </div>
          ))}
        </div>
        {/* 编辑区域 */}
        <textarea
          value={code}
          onChange={(e) => onContentChange?.(e.target.value)}
          className="flex-1 p-3 font-mono text-sm leading-6 bg-transparent resize-none outline-none"
          spellCheck={false}
        />
      </div>
    );
  }

  return (
    <div className="h-full flex overflow-auto">
      {/* 行号 */}
      <div
        className="flex-shrink-0 bg-ink-50 border-r border-ink-200 text-right py-3 select-none sticky left-0"
        style={{ width: lineNumberWidth }}
      >
        {lines.map((_, index) => (
          <div
            key={index}
            className="px-2 text-xs text-ink-400 leading-6 font-mono"
          >
            {index + 1}
          </div>
        ))}
      </div>
      {/* 代码内容 */}
      <div className="flex-1 overflow-x-auto">
        <pre className="p-3 font-mono text-sm leading-6 text-ink-800">
          <code>{code}</code>
        </pre>
      </div>
    </div>
  );
};

export default CodePreview;
