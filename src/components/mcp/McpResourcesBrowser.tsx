/**
 * MCP 资源浏览器组件
 *
 * 按服务器分组显示所有可用的 MCP 资源，支持资源内容预览。
 *
 * @module components/mcp/McpResourcesBrowser
 */

import { useState } from "react";
import {
  FileText,
  ChevronDown,
  ChevronRight,
  RefreshCw,
  Search,
  Eye,
  X,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { McpResourceDefinition, McpResourceContent } from "@/lib/api/mcp";

interface McpResourcesBrowserProps {
  resources: McpResourceDefinition[];
  loading: boolean;
  onRefresh: () => Promise<void>;
  onReadResource: (uri: string) => Promise<McpResourceContent>;
}

export function McpResourcesBrowser({
  resources,
  loading,
  onRefresh,
  onReadResource,
}: McpResourcesBrowserProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedServers, setExpandedServers] = useState<Set<string>>(
    new Set(),
  );
  const [activeResource, setActiveResource] = useState<string | null>(null);
  const [resourceContent, setResourceContent] =
    useState<McpResourceContent | null>(null);
  const [reading, setReading] = useState(false);
  const [readError, setReadError] = useState<string | null>(null);

  // 按服务器分组
  const resourcesByServer = resources.reduce(
    (acc, res) => {
      if (!acc[res.server_name]) acc[res.server_name] = [];
      acc[res.server_name].push(res);
      return acc;
    },
    {} as Record<string, McpResourceDefinition[]>,
  );

  // 过滤
  const filteredByServer = Object.entries(resourcesByServer).reduce(
    (acc, [serverName, serverResources]) => {
      const filtered = serverResources.filter(
        (r) =>
          r.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          r.uri.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (r.description || "")
            .toLowerCase()
            .includes(searchQuery.toLowerCase()),
      );
      if (filtered.length > 0) acc[serverName] = filtered;
      return acc;
    },
    {} as Record<string, McpResourceDefinition[]>,
  );

  const toggleServer = (name: string) => {
    const s = new Set(expandedServers);
    if (s.has(name)) {
      s.delete(name);
    } else {
      s.add(name);
    }
    setExpandedServers(s);
  };

  const handleReadResource = async (uri: string) => {
    if (activeResource === uri) {
      setActiveResource(null);
      setResourceContent(null);
      return;
    }
    setActiveResource(uri);
    setReading(true);
    setReadError(null);
    setResourceContent(null);
    try {
      const content = await onReadResource(uri);
      setResourceContent(content);
    } catch (e) {
      setReadError(e instanceof Error ? e.message : String(e));
    } finally {
      setReading(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* 标题栏 */}
      <div className="p-3 border-b flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FileText className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">资源</span>
          <span className="text-xs text-muted-foreground">
            ({resources.length})
          </span>
        </div>
        <button
          onClick={() => onRefresh()}
          disabled={loading}
          className="p-1.5 rounded hover:bg-muted"
          title="刷新资源列表"
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
            placeholder="搜索资源..."
            className="w-full pl-8 pr-3 py-1.5 rounded border bg-background text-sm focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none"
          />
        </div>
      </div>

      {/* 资源列表 */}
      <div className="flex-1 overflow-auto">
        {loading && resources.length === 0 ? (
          <div className="flex items-center justify-center py-8">
            <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        ) : Object.keys(filteredByServer).length === 0 ? (
          <div className="text-center py-8 text-muted-foreground text-sm">
            {searchQuery
              ? "未找到匹配的资源"
              : "暂无可用资源，请先启动 MCP 服务器"}
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {Object.entries(filteredByServer).map(
              ([serverName, serverResources]) => (
                <div key={serverName} className="border rounded-lg">
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
                      ({serverResources.length} 个资源)
                    </span>
                  </button>

                  {expandedServers.has(serverName) && (
                    <div className="border-t">
                      {serverResources.map((resource) => (
                        <div
                          key={resource.uri}
                          className="border-b last:border-b-0"
                        >
                          <div className="p-2.5 pl-8 flex items-start justify-between gap-2">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <FileText className="h-3.5 w-3.5 text-orange-500 flex-shrink-0" />
                                <span className="font-medium text-sm truncate">
                                  {resource.name}
                                </span>
                              </div>
                              <p className="text-xs text-muted-foreground mt-0.5 font-mono truncate">
                                {resource.uri}
                              </p>
                              {resource.description && (
                                <p className="text-xs text-muted-foreground mt-0.5 line-clamp-1">
                                  {resource.description}
                                </p>
                              )}
                              {resource.mime_type && (
                                <span className="inline-block mt-1 px-1.5 py-0.5 text-xs rounded bg-muted text-muted-foreground">
                                  {resource.mime_type}
                                </span>
                              )}
                            </div>
                            <button
                              onClick={() => handleReadResource(resource.uri)}
                              className="p-1 rounded hover:bg-muted text-muted-foreground flex-shrink-0"
                              title="读取资源"
                            >
                              {activeResource === resource.uri ? (
                                <X className="h-4 w-4" />
                              ) : (
                                <Eye className="h-4 w-4" />
                              )}
                            </button>
                          </div>

                          {/* 资源内容预览 */}
                          {activeResource === resource.uri && (
                            <div className="px-8 pb-3">
                              {reading ? (
                                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                                  <RefreshCw className="h-3 w-3 animate-spin" />
                                  读取中...
                                </div>
                              ) : readError ? (
                                <div className="p-2 rounded bg-destructive/10 text-destructive text-xs">
                                  {readError}
                                </div>
                              ) : resourceContent ? (
                                <div className="bg-muted/50 rounded-lg p-3">
                                  <div className="flex items-center gap-2 mb-2">
                                    <span className="text-xs font-medium text-muted-foreground">
                                      {resourceContent.mime_type ||
                                        "text/plain"}
                                    </span>
                                  </div>
                                  {resourceContent.text ? (
                                    <pre className="text-xs font-mono overflow-x-auto whitespace-pre-wrap break-all bg-background p-2 rounded border max-h-64 overflow-y-auto">
                                      {resourceContent.text}
                                    </pre>
                                  ) : resourceContent.blob ? (
                                    <div className="text-xs text-muted-foreground">
                                      [二进制数据, {resourceContent.blob.length}{" "}
                                      字节]
                                    </div>
                                  ) : (
                                    <div className="text-xs text-muted-foreground">
                                      无内容
                                    </div>
                                  )}
                                </div>
                              ) : null}
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
