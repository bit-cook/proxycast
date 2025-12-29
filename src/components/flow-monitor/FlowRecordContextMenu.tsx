/**
 * Flow 记录右键菜单组件
 *
 * 为 Flow Monitor 的请求记录提供右键菜单功能
 * 支持查看详情、复制 ID、复制为 cURL、导出 JSON 等操作
 *
 * @module components/flow-monitor/FlowRecordContextMenu
 */

import React from "react";
import { ExternalLink, Copy, Terminal, FileJson } from "lucide-react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { toast } from "sonner";
import type { LLMFlow } from "@/lib/api/flowMonitor";

interface FlowRecordContextMenuProps {
  /** Flow 记录数据 */
  flow: LLMFlow;
  /** 子元素 */
  children: React.ReactNode;
  /** 查看详情回调 */
  onViewDetail: () => void;
  /** 导出 JSON 回调 */
  onExportJson?: (flowId: string) => void;
}

/**
 * 生成 cURL 命令
 */
function generateCurlCommand(flow: LLMFlow): string {
  const { request, metadata } = flow;

  // 基础 URL（根据 provider 推断）
  const baseUrls: Record<string, string> = {
    Kiro: "https://codewhisperer.us-east-1.amazonaws.com",
    OpenAI: "https://api.openai.com/v1/chat/completions",
    Claude: "https://api.anthropic.com/v1/messages",
    Gemini: "https://generativelanguage.googleapis.com/v1beta/models",
    Qwen: "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation",
  };

  const url =
    baseUrls[metadata.provider] ||
    "https://api.example.com/v1/chat/completions";

  // 构建请求体（避免 stream 重复）
  const { stream: _stream, ...otherParams } = request.parameters;
  const body = {
    model: request.model,
    messages: request.messages,
    stream: request.parameters.stream,
    ...otherParams,
  };

  // 构建 cURL 命令
  const parts = [
    "curl",
    `-X ${request.method || "POST"}`,
    `'${url}'`,
    "-H 'Content-Type: application/json'",
    "-H 'Authorization: Bearer YOUR_API_KEY'",
    `-d '${JSON.stringify(body, null, 2)}'`,
  ];

  return parts.join(" \\\n  ");
}

export function FlowRecordContextMenu({
  flow,
  children,
  onViewDetail,
  onExportJson,
}: FlowRecordContextMenuProps) {
  // 复制请求 ID
  const handleCopyId = async () => {
    try {
      await navigator.clipboard.writeText(flow.id);
      toast.success("已复制请求 ID");
    } catch (error) {
      console.error("复制失败:", error);
      toast.error("复制失败");
    }
  };

  // 复制为 cURL
  const handleCopyAsCurl = async () => {
    try {
      const curlCommand = generateCurlCommand(flow);
      await navigator.clipboard.writeText(curlCommand);
      toast.success("已复制 cURL 命令");
    } catch (error) {
      console.error("复制失败:", error);
      toast.error("复制失败");
    }
  };

  // 导出为 JSON
  const handleExportJson = () => {
    if (onExportJson) {
      onExportJson(flow.id);
    } else {
      // 默认导出行为：下载 JSON 文件
      const jsonStr = JSON.stringify(flow, null, 2);
      const blob = new Blob([jsonStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `flow-${flow.id.slice(0, 8)}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success("已导出 JSON 文件");
    }
  };

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent className="w-48">
        {/* 查看详情 */}
        <ContextMenuItem onClick={onViewDetail}>
          <ExternalLink className="mr-2 h-4 w-4" />
          查看详情
          <ContextMenuShortcut>↵</ContextMenuShortcut>
        </ContextMenuItem>

        {/* 复制请求 ID */}
        <ContextMenuItem onClick={handleCopyId}>
          <Copy className="mr-2 h-4 w-4" />
          复制请求 ID
          <ContextMenuShortcut>C</ContextMenuShortcut>
        </ContextMenuItem>

        {/* 复制为 cURL */}
        <ContextMenuItem onClick={handleCopyAsCurl}>
          <Terminal className="mr-2 h-4 w-4" />
          复制为 cURL
          <ContextMenuShortcut>⇧C</ContextMenuShortcut>
        </ContextMenuItem>

        <ContextMenuSeparator />

        {/* 导出为 JSON */}
        <ContextMenuItem onClick={handleExportJson}>
          <FileJson className="mr-2 h-4 w-4" />
          导出为 JSON
          <ContextMenuShortcut>E</ContextMenuShortcut>
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
