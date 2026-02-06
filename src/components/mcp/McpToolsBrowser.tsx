/**
 * MCP 工具浏览器组件
 *
 * 按服务器分组显示所有可用的 MCP 工具，包括工具名称、描述和参数 schema。
 *
 * @module components/mcp/McpToolsBrowser
 */

import { useState } from "react";
import {
  Wrench,
  ChevronDown,
  ChevronRight,
  RefreshCw,
  Search,
  Code,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { McpToolDefinition } from "@/lib/api/mcp";

interface McpToolsBrowserProps {
  tools: McpToolDefinition[];
  loading: boolean;
  onRefresh: () => Promise<void>;
  onCallTool?: (
    toolName: string,
    args: Record<string, unknown>,
  ) => Promise<void>;
}

export function McpToolsBrowser({
  tools,
  loading,
  onRefresh,
  onCallTool,
}: McpToolsBrowserProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedServers, setExpandedServers] = useState<Set<string>>(
    new Set(),
  );
  const [expandedTools, setExpandedTools] = useState<Set<string>>(new Set());

  // 按服务器分组工具
  const toolsByServer = tools.reduce(
    (acc, tool) => {
      if (!acc[tool.server_name]) {
        acc[tool.server_name] = [];
      }
      acc[tool.server_name].push(tool);
      return acc;
    },
    {} as Record<string, McpToolDefinition[]>,
  );

  // 过滤工具
  const filteredToolsByServer = Object.entries(toolsByServer).reduce(
    (acc, [serverName, serverTools]) => {
      const filtered = serverTools.filter(
        (tool) =>
          tool.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          tool.description.toLowerCase().includes(searchQuery.toLowerCase()),
      );
      if (filtered.length > 0) {
        acc[serverName] = filtered;
      }
      return acc;
    },
    {} as Record<string, McpToolDefinition[]>,
  );

  const toggleServer = (serverName: string) => {
    const newExpanded = new Set(expandedServers);
    if (newExpanded.has(serverName)) {
      newExpanded.delete(serverName);
    } else {
      newExpanded.add(serverName);
    }
    setExpandedServers(newExpanded);
  };

  const toggleTool = (toolName: string) => {
    const newExpanded = new Set(expandedTools);
    if (newExpanded.has(toolName)) {
      newExpanded.delete(toolName);
    } else {
      newExpanded.add(toolName);
    }
    setExpandedTools(newExpanded);
  };

  // 格式化 JSON Schema
  const formatSchema = (schema: Record<string, unknown>) => {
    return JSON.stringify(schema, null, 2);
  };

  return (
    <div className="flex flex-col h-full">
      {/* 标题栏 */}
      <div className="p-3 border-b flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Wrench className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">可用工具</span>
          <span className="text-xs text-muted-foreground">
            ({tools.length})
          </span>
        </div>
        <button
          onClick={() => onRefresh()}
          disabled={loading}
          className="p-1.5 rounded hover:bg-muted"
          title="刷新工具列表"
        >
          <RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />
        </button>
      </div>

      {/* 搜索框 */}
      <div className="p-2 border-b">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="搜索工具..."
            className="w-full pl-8 pr-3 py-1.5 rounded border bg-background text-sm focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none"
          />
        </div>
      </div>

      {/* 工具列表 */}
      <div className="flex-1 overflow-auto">
        {loading && tools.length === 0 ? (
          <div className="flex items-center justify-center py-8">
            <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        ) : Object.keys(filteredToolsByServer).length === 0 ? (
          <div className="text-center py-8 text-muted-foreground text-sm">
            {searchQuery ? (
              <p>未找到匹配的工具</p>
            ) : (
              <p>暂无可用工具，请先启动 MCP 服务器</p>
            )}
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {Object.entries(filteredToolsByServer).map(
              ([serverName, serverTools]) => (
                <div key={serverName} className="border rounded-lg">
                  {/* 服务器标题 */}
                  <button
                    onClick={() => toggleServer(serverName)}
                    className="w-full p-2.5 flex items-center gap-2 hover:bg-muted/50 rounded-t-lg"
                  >
                    {expandedServers.has(serverName) ? (
                      <ChevronDown className="h-4 w-4 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="h-4 w-4 text-muted-foreground" />
                    )}
                    <span className="font-medium text-sm">{serverName}</span>
                    <span className="text-xs text-muted-foreground">
                      ({serverTools.length} 个工具)
                    </span>
                  </button>

                  {/* 工具列表 */}
                  {expandedServers.has(serverName) && (
                    <div className="border-t">
                      {serverTools.map((tool) => (
                        <div
                          key={tool.name}
                          className="border-b last:border-b-0"
                        >
                          {/* 工具标题 */}
                          <button
                            onClick={() => toggleTool(tool.name)}
                            className="w-full p-2.5 pl-8 flex items-start gap-2 hover:bg-muted/30 text-left"
                          >
                            {expandedTools.has(tool.name) ? (
                              <ChevronDown className="h-4 w-4 text-muted-foreground mt-0.5 flex-shrink-0" />
                            ) : (
                              <ChevronRight className="h-4 w-4 text-muted-foreground mt-0.5 flex-shrink-0" />
                            )}
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <Code className="h-3.5 w-3.5 text-blue-500 flex-shrink-0" />
                                <span className="font-mono text-sm text-blue-600 dark:text-blue-400">
                                  {tool.name}
                                </span>
                              </div>
                              {tool.description && (
                                <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
                                  {tool.description}
                                </p>
                              )}
                            </div>
                          </button>

                          {/* 工具详情 */}
                          {expandedTools.has(tool.name) && (
                            <div className="px-8 pb-3">
                              <div className="bg-muted/50 rounded-lg p-3">
                                <div className="flex items-center justify-between mb-2">
                                  <span className="text-xs font-medium text-muted-foreground">
                                    输入参数 Schema
                                  </span>
                                  {onCallTool && (
                                    <button
                                      onClick={() => onCallTool(tool.name, {})}
                                      className="px-2 py-1 text-xs rounded bg-blue-600 text-white hover:bg-blue-700"
                                    >
                                      调用工具
                                    </button>
                                  )}
                                </div>
                                <pre className="text-xs font-mono overflow-x-auto whitespace-pre-wrap break-all bg-background p-2 rounded border">
                                  {formatSchema(tool.input_schema)}
                                </pre>
                              </div>
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              ),
            )}
          </div>
        )}
      </div>
    </div>
  );
}
