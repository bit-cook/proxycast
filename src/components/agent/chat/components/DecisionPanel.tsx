/**
 * DecisionPanel - 权限确认面板
 *
 * 用于显示需要用户确认的操作，如：
 * - 工具调用确认
 * - 用户问题（AskUserQuestion）
 * - 权限请求
 *
 * 参考 Claude-Cowork 的设计
 */

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import {
  CheckCircle,
  XCircle,
  AlertTriangle,
  HelpCircle,
  Terminal,
  FileEdit,
  Globe,
} from "lucide-react";
import type { ActionRequired, ConfirmResponse } from "../types";

interface DecisionPanelProps {
  request: ActionRequired;
  onSubmit: (response: ConfirmResponse) => void;
}

/** 获取工具图标 */
function getToolIcon(toolName?: string) {
  if (!toolName) return <HelpCircle className="h-4 w-4" />;

  const name = toolName.toLowerCase();
  if (
    name.includes("bash") ||
    name.includes("terminal") ||
    name.includes("exec")
  ) {
    return <Terminal className="h-4 w-4" />;
  }
  if (
    name.includes("write") ||
    name.includes("edit") ||
    name.includes("file")
  ) {
    return <FileEdit className="h-4 w-4" />;
  }
  if (name.includes("web") || name.includes("fetch") || name.includes("http")) {
    return <Globe className="h-4 w-4" />;
  }
  return <AlertTriangle className="h-4 w-4" />;
}

/** 格式化工具参数 */
function formatArguments(args?: Record<string, unknown>): string {
  if (!args) return "";
  try {
    return JSON.stringify(args, null, 2);
  } catch {
    return String(args);
  }
}

export function DecisionPanel({ request, onSubmit }: DecisionPanelProps) {
  // 解析问题数据（用于 ask_user 类型）
  const questions = request.questions || [];
  const [selectedOptions, setSelectedOptions] = useState<
    Record<number, string[]>
  >({});
  const [otherInputs, setOtherInputs] = useState<Record<number, string>>({});

  // 重置状态当请求变化时
  useEffect(() => {
    setSelectedOptions({});
    setOtherInputs({});
  }, [request.requestId]);

  // 切换选项
  const toggleOption = (
    qIndex: number,
    optionLabel: string,
    multiSelect?: boolean,
  ) => {
    setSelectedOptions((prev) => {
      const current = prev[qIndex] ?? [];
      if (multiSelect) {
        const next = current.includes(optionLabel)
          ? current.filter((label) => label !== optionLabel)
          : [...current, optionLabel];
        return { ...prev, [qIndex]: next };
      }
      return { ...prev, [qIndex]: [optionLabel] };
    });
  };

  // 构建答案
  const buildAnswers = () => {
    const answers: Record<string, string> = {};
    questions.forEach((q, qIndex) => {
      const selected = selectedOptions[qIndex] ?? [];
      const otherText = otherInputs[qIndex]?.trim() ?? "";
      let value = "";
      if (q.multiSelect) {
        const combined = [...selected];
        if (otherText) combined.push(otherText);
        value = combined.join(", ");
      } else {
        value = otherText || selected[0] || "";
      }
      if (value) answers[q.question] = value;
    });
    return answers;
  };

  // 检查是否���以提交
  const canSubmit =
    questions.length === 0 ||
    questions.every((_, qIndex) => {
      const selected = selectedOptions[qIndex] ?? [];
      const otherText = otherInputs[qIndex]?.trim() ?? "";
      return selected.length > 0 || otherText.length > 0;
    });

  // 处理允许
  const handleAllow = () => {
    const response =
      questions.length > 0 ? JSON.stringify(buildAnswers()) : undefined;
    onSubmit({
      requestId: request.requestId,
      confirmed: true,
      response,
    });
  };

  // 处理拒绝
  const handleDeny = () => {
    onSubmit({
      requestId: request.requestId,
      confirmed: false,
      response: "用户拒绝了请求",
    });
  };

  // 渲染用户问题面板
  if (
    request.actionType === "ask_user" &&
    request.questions &&
    request.questions.length > 0
  ) {
    const questions = request.questions;
    return (
      <Card className="border-blue-200 bg-blue-50/50 dark:border-blue-800 dark:bg-blue-950/20">
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-sm font-medium text-blue-700 dark:text-blue-300">
            <HelpCircle className="h-4 w-4" />
            Claude 的问题
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {questions.map((q, qIndex) => (
            <div key={qIndex} className="space-y-3">
              <p className="text-sm text-foreground">{q.question}</p>

              {q.header && (
                <Badge variant="secondary" className="text-xs">
                  {q.header}
                </Badge>
              )}

              {/* 选项列表 */}
              {q.options && q.options.length > 0 && (
                <div className="grid gap-2">
                  {q.options.map((option, optIndex) => {
                    const isSelected = (selectedOptions[qIndex] ?? []).includes(
                      option.label,
                    );
                    const shouldAutoSubmit =
                      questions.length === 1 && !q.multiSelect;

                    return (
                      <button
                        key={optIndex}
                        className={cn(
                          "rounded-lg border px-4 py-3 text-left text-sm transition-colors",
                          isSelected
                            ? "border-blue-500 bg-blue-100 dark:border-blue-400 dark:bg-blue-900/30"
                            : "border-border bg-background hover:border-blue-300 hover:bg-muted",
                        )}
                        onClick={() => {
                          if (shouldAutoSubmit) {
                            onSubmit({
                              requestId: request.requestId,
                              confirmed: true,
                              response: option.label,
                            });
                            return;
                          }
                          toggleOption(qIndex, option.label, q.multiSelect);
                        }}
                      >
                        <div className="font-medium">{option.label}</div>
                        {option.description && (
                          <div className="mt-1 text-xs text-muted-foreground">
                            {option.description}
                          </div>
                        )}
                      </button>
                    );
                  })}
                </div>
              )}

              {/* 其他输入 */}
              <div className="space-y-1">
                <label className="text-xs font-medium text-muted-foreground">
                  其他
                </label>
                <Input
                  placeholder="输入你的答案..."
                  value={otherInputs[qIndex] ?? ""}
                  onChange={(e) =>
                    setOtherInputs((prev) => ({
                      ...prev,
                      [qIndex]: e.target.value,
                    }))
                  }
                />
              </div>

              {q.multiSelect && (
                <p className="text-xs text-muted-foreground">
                  可以选择多个选项
                </p>
              )}
            </div>
          ))}

          {/* 操作按钮 */}
          <div className="flex gap-2 pt-2">
            <Button
              size="sm"
              onClick={handleAllow}
              disabled={!canSubmit}
              className="bg-blue-600 hover:bg-blue-700"
            >
              <CheckCircle className="mr-1 h-4 w-4" />
              提交答案
            </Button>
            <Button size="sm" variant="outline" onClick={handleDeny}>
              <XCircle className="mr-1 h-4 w-4" />
              取消
            </Button>
          </div>
        </CardContent>
      </Card>
    );
  }

  // 渲染工具确认面板
  return (
    <Card className="border-amber-200 bg-amber-50/50 dark:border-amber-800 dark:bg-amber-950/20">
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm font-medium text-amber-700 dark:text-amber-300">
          <AlertTriangle className="h-4 w-4" />
          权限请求
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {/* 工具信息 */}
        <div className="flex items-center gap-2">
          {getToolIcon(request.toolName)}
          <span className="text-sm">
            Claude 想要使用：
            <span className="ml-1 font-medium">
              {request.toolName || "未知工具"}
            </span>
          </span>
        </div>

        {/* 参数预览 */}
        {request.arguments && (
          <div className="rounded-lg bg-muted/50 p-3">
            <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-words font-mono text-xs text-muted-foreground">
              {formatArguments(request.arguments)}
            </pre>
          </div>
        )}

        {/* 操作按钮 */}
        <div className="flex gap-2 pt-2">
          <Button
            size="sm"
            onClick={handleAllow}
            className="bg-green-600 hover:bg-green-700"
          >
            <CheckCircle className="mr-1 h-4 w-4" />
            允许
          </Button>
          <Button size="sm" variant="outline" onClick={handleDeny}>
            <XCircle className="mr-1 h-4 w-4" />
            拒绝
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

/** 权限确认列表组件 */
export function DecisionPanelList({
  requests,
  onSubmit,
}: {
  requests: ActionRequired[];
  onSubmit: (response: ConfirmResponse) => void;
}) {
  if (requests.length === 0) return null;

  return (
    <div className="space-y-3">
      {requests.map((request) => (
        <DecisionPanel
          key={request.requestId}
          request={request}
          onSubmit={onSubmit}
        />
      ))}
    </div>
  );
}

export default DecisionPanel;
