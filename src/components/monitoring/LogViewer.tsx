import React, { useState } from "react";
import {
  FileText,
  CheckCircle2,
  XCircle,
  Clock,
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Trash2,
  Filter,
} from "lucide-react";
import type { RequestLog, RequestStatus } from "@/lib/api/telemetry";

interface LogViewerProps {
  logs: RequestLog[];
  onClear: () => void;
  onFilter?: (filter: { provider?: string; status?: RequestStatus }) => void;
}

export function LogViewer({ logs, onClear, onFilter }: LogViewerProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [filterProvider, setFilterProvider] = useState<string>("");
  const [filterStatus, setFilterStatus] = useState<string>("");

  const getStatusIcon = (status: RequestStatus) => {
    switch (status) {
      case "success":
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case "failed":
        return <XCircle className="h-4 w-4 text-red-500" />;
      case "timeout":
        return <Clock className="h-4 w-4 text-yellow-500" />;
      case "retrying":
        return <AlertTriangle className="h-4 w-4 text-orange-500" />;
      case "cancelled":
        return <XCircle className="h-4 w-4 text-gray-500" />;
    }
  };

  const getStatusText = (status: RequestStatus) => {
    const texts: Record<RequestStatus, string> = {
      success: "成功",
      failed: "失败",
      timeout: "超时",
      retrying: "重试中",
      cancelled: "已取消",
    };
    return texts[status];
  };

  const getProviderName = (id: string) => {
    const names: Record<string, string> = {
      kiro: "Kiro",
      gemini: "Gemini",
      qwen: "Qwen",
      openai: "OpenAI",
      claude: "Claude",
      antigravity: "Antigravity",
    };
    return names[id] || id;
  };

  const formatTime = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const handleFilterChange = () => {
    onFilter?.({
      provider: filterProvider || undefined,
      status: (filterStatus as RequestStatus) || undefined,
    });
  };

  // 获取唯一的 providers
  const providers = [...new Set(logs.map((l) => l.provider))];

  return (
    <div className="space-y-4">
      {/* 过滤器和操作 */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Filter className="h-4 w-4 text-muted-foreground" />
          <select
            className="rounded border bg-background px-2 py-1 text-sm"
            value={filterProvider}
            onChange={(e) => {
              setFilterProvider(e.target.value);
              handleFilterChange();
            }}
          >
            <option value="">所有 Provider</option>
            {providers.map((p) => (
              <option key={p} value={p}>
                {getProviderName(p)}
              </option>
            ))}
          </select>
          <select
            className="rounded border bg-background px-2 py-1 text-sm"
            value={filterStatus}
            onChange={(e) => {
              setFilterStatus(e.target.value);
              handleFilterChange();
            }}
          >
            <option value="">所有状态</option>
            <option value="success">成功</option>
            <option value="failed">失败</option>
            <option value="timeout">超时</option>
          </select>
        </div>
        <button
          onClick={onClear}
          className="flex items-center gap-1 rounded px-2 py-1 text-sm text-muted-foreground hover:bg-muted"
        >
          <Trash2 className="h-3 w-3" />
          清空日志
        </button>
      </div>

      {/* 日志列表 */}
      <div className="rounded-lg border bg-card">
        <div className="border-b px-4 py-2 flex items-center gap-2">
          <FileText className="h-4 w-4" />
          <span className="font-medium">请求日志</span>
          <span className="text-sm text-muted-foreground">
            ({logs.length} 条)
          </span>
        </div>

        {logs.length === 0 ? (
          <div className="p-8 text-center text-muted-foreground">
            暂无请求日志
          </div>
        ) : (
          <div className="divide-y max-h-[500px] overflow-y-auto">
            {logs.map((log) => (
              <LogEntry
                key={log.id}
                log={log}
                expanded={expandedId === log.id}
                onToggle={() =>
                  setExpandedId(expandedId === log.id ? null : log.id)
                }
                getStatusIcon={getStatusIcon}
                getStatusText={getStatusText}
                getProviderName={getProviderName}
                formatTime={formatTime}
                formatDuration={formatDuration}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function LogEntry({
  log,
  expanded,
  onToggle,
  getStatusIcon,
  getStatusText,
  getProviderName,
  formatTime,
  formatDuration,
}: {
  log: RequestLog;
  expanded: boolean;
  onToggle: () => void;
  getStatusIcon: (status: RequestStatus) => React.ReactNode;
  getStatusText: (status: RequestStatus) => string;
  getProviderName: (id: string) => string;
  formatTime: (timestamp: string) => string;
  formatDuration: (ms: number) => string;
}) {
  return (
    <div className="hover:bg-muted/50">
      <div
        className="flex items-center gap-3 px-4 py-2 cursor-pointer"
        onClick={onToggle}
      >
        {expanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
        {getStatusIcon(log.status)}
        <span className="text-xs text-muted-foreground w-32">
          {formatTime(log.timestamp)}
        </span>
        <span className="font-medium w-20">
          {getProviderName(log.provider)}
        </span>
        <span className="text-sm flex-1 truncate">{log.model}</span>
        <span className="text-sm text-muted-foreground w-16 text-right">
          {formatDuration(log.duration_ms)}
        </span>
        {log.total_tokens && (
          <span className="text-xs text-muted-foreground w-20 text-right">
            {log.total_tokens} tokens
          </span>
        )}
      </div>

      {expanded && (
        <div className="px-4 pb-3 pl-12 space-y-2 text-sm">
          <div className="grid grid-cols-2 gap-4 rounded bg-muted/50 p-3">
            <div>
              <span className="text-muted-foreground">状态:</span>{" "}
              <span
                className={
                  log.status === "success" ? "text-green-600" : "text-red-600"
                }
              >
                {getStatusText(log.status)}
              </span>
              {log.http_status && (
                <span className="ml-2 text-muted-foreground">
                  (HTTP {log.http_status})
                </span>
              )}
            </div>
            <div>
              <span className="text-muted-foreground">流式:</span>{" "}
              {log.is_streaming ? "是" : "否"}
            </div>
            {log.input_tokens !== undefined && (
              <div>
                <span className="text-muted-foreground">输入 Token:</span>{" "}
                {log.input_tokens}
              </div>
            )}
            {log.output_tokens !== undefined && (
              <div>
                <span className="text-muted-foreground">输出 Token:</span>{" "}
                {log.output_tokens}
              </div>
            )}
            {log.retry_count > 0 && (
              <div>
                <span className="text-muted-foreground">重试次数:</span>{" "}
                {log.retry_count}
              </div>
            )}
            {log.credential_id && (
              <div>
                <span className="text-muted-foreground">凭证 ID:</span>{" "}
                <code className="text-xs">
                  {log.credential_id.slice(0, 8)}...
                </code>
              </div>
            )}
          </div>
          {log.error_message && (
            <div className="rounded bg-red-50 dark:bg-red-950/20 p-2 text-red-600 dark:text-red-400">
              <span className="font-medium">错误:</span> {log.error_message}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
