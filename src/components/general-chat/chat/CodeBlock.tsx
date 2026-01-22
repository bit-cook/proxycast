/**
 * @file CodeBlock.tsx
 * @description 代码块组件 - 显示代码，支持语法高亮和操作按钮
 * @module components/general-chat/chat/CodeBlock
 *
 * @requirements 2.4, 6.3
 */

import React, { useState } from "react";

interface CodeBlockProps {
  /** 代码内容 */
  code: string;
  /** 代码语言 */
  language: string;
  /** 文件名 (可选) */
  filename?: string;
  /** 复制回调 */
  onCopy: () => void;
  /** 在画布中打开回调 */
  onOpenInCanvas: () => void;
}

/**
 * 获取语言显示名称
 */
const getLanguageDisplayName = (lang: string): string => {
  const names: Record<string, string> = {
    javascript: "JavaScript",
    typescript: "TypeScript",
    python: "Python",
    rust: "Rust",
    go: "Go",
    java: "Java",
    cpp: "C++",
    c: "C",
    csharp: "C#",
    ruby: "Ruby",
    php: "PHP",
    swift: "Swift",
    kotlin: "Kotlin",
    html: "HTML",
    css: "CSS",
    scss: "SCSS",
    json: "JSON",
    yaml: "YAML",
    xml: "XML",
    sql: "SQL",
    bash: "Bash",
    shell: "Shell",
    markdown: "Markdown",
    plaintext: "Text",
  };
  return names[lang.toLowerCase()] || lang;
};

/**
 * 代码块组件
 */
export const CodeBlock: React.FC<CodeBlockProps> = ({
  code,
  language,
  filename,
  onCopy,
  onOpenInCanvas,
}) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    onCopy();
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="rounded-lg overflow-hidden border border-ink-200 bg-ink-50">
      {/* 头部工具栏 */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-ink-100 border-b border-ink-200">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-ink-600">
            {filename || getLanguageDisplayName(language)}
          </span>
        </div>
        <div className="flex items-center gap-1">
          {/* 复制按钮 */}
          <button
            onClick={handleCopy}
            className="p-1 text-ink-500 hover:text-ink-700 transition-colors"
            title={copied ? "已复制" : "复制代码"}
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
          {/* 在画布中打开按钮 */}
          <button
            onClick={onOpenInCanvas}
            className="p-1 text-ink-500 hover:text-ink-700 transition-colors"
            title="在画布中打开"
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
                d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
              />
            </svg>
          </button>
        </div>
      </div>

      {/* 代码内容 */}
      <div className="overflow-x-auto">
        <pre className="p-3 text-sm font-mono text-ink-800 leading-relaxed">
          <code>{code}</code>
        </pre>
      </div>
    </div>
  );
};

export default CodeBlock;
