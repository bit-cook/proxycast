/**
 * 外部工具设置组件
 *
 * 管理 Codex CLI 等外部命令行工具的状态和配置
 * 这些工具有自己的认证系统，不通过 ProxyCast 凭证池管理
 *
 * @module components/settings-v2/system/external-tools
 */

import { useState, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  RefreshCw,
  CheckCircle,
  XCircle,
  ExternalLink,
  Terminal,
  Copy,
  AlertCircle,
} from "lucide-react";
import {
  checkCodexCliStatus,
  getCodexLoginCommand,
  getCodexLogoutCommand,
  type CodexCliStatus,
} from "@/lib/api/externalTools";
import { toast } from "sonner";

export function ExternalToolsSettings() {
  const [codexStatus, setCodexStatus] = useState<CodexCliStatus | null>(null);
  const [loading, setLoading] = useState(true);

  // 加载 Codex CLI 状态
  const loadStatus = useCallback(async () => {
    setLoading(true);
    try {
      const status = await checkCodexCliStatus();
      setCodexStatus(status);
    } catch (err) {
      console.error("[ExternalTools] 加载状态失败:", err);
      setCodexStatus({
        installed: false,
        logged_in: false,
        error: String(err),
      });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  // 复制命令到剪贴板
  const copyCommand = async (command: string) => {
    await navigator.clipboard.writeText(command);
    toast.success("命令已复制到剪贴板，请在终端中执行");
  };

  // 处理登录
  const handleLogin = async () => {
    const cmd = await getCodexLoginCommand();
    await copyCommand(cmd);
  };

  // 处理登出
  const handleLogout = async () => {
    const cmd = await getCodexLogoutCommand();
    await copyCommand(cmd);
  };

  return (
    <div className="space-y-6 max-w-2xl">
      {/* Codex CLI */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Terminal className="w-5 h-5" />
              <span>Codex CLI</span>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={loadStatus}
              disabled={loading}
            >
              <RefreshCw
                className={`w-4 h-4 ${loading ? "animate-spin" : ""}`}
              />
            </Button>
          </CardTitle>
          <CardDescription>
            OpenAI Codex 命令行工具，用于 Agent 模式的代码生成和工具调用
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* 状态显示 */}
          {codexStatus && (
            <div className="space-y-3">
              {/* 安装状态 */}
              <div className="flex items-center justify-between p-3 bg-muted/50 rounded-md">
                <div className="flex items-center gap-2">
                  {codexStatus.installed ? (
                    <CheckCircle className="w-4 h-4 text-green-500" />
                  ) : (
                    <XCircle className="w-4 h-4 text-red-500" />
                  )}
                  <span className="text-sm">
                    {codexStatus.installed ? "已安装" : "未安装"}
                  </span>
                  {codexStatus.version && (
                    <code className="text-xs bg-muted px-2 py-0.5 rounded">
                      {codexStatus.version}
                    </code>
                  )}
                </div>
                {!codexStatus.installed && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => copyCommand("npm i -g @openai/codex")}
                  >
                    <Copy className="w-3 h-3 mr-1" />
                    复制安装命令
                  </Button>
                )}
              </div>

              {/* 登录状态 */}
              {codexStatus.installed && (
                <div className="flex items-center justify-between p-3 bg-muted/50 rounded-md">
                  <div className="flex items-center gap-2">
                    {codexStatus.logged_in ? (
                      <CheckCircle className="w-4 h-4 text-green-500" />
                    ) : (
                      <AlertCircle className="w-4 h-4 text-yellow-500" />
                    )}
                    <span className="text-sm">
                      {codexStatus.logged_in ? "已登录" : "未登录"}
                    </span>
                    {codexStatus.auth_type && (
                      <code className="text-xs bg-muted px-2 py-0.5 rounded">
                        {codexStatus.auth_type === "api_key"
                          ? "API Key"
                          : codexStatus.auth_type === "oauth"
                            ? "OAuth"
                            : codexStatus.auth_type}
                      </code>
                    )}
                    {codexStatus.api_key_prefix && (
                      <code className="text-xs bg-muted px-2 py-0.5 rounded text-muted-foreground">
                        {codexStatus.api_key_prefix}
                      </code>
                    )}
                  </div>
                  <div className="flex gap-2">
                    {codexStatus.logged_in ? (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleLogout}
                      >
                        登出
                      </Button>
                    ) : (
                      <Button variant="default" size="sm" onClick={handleLogin}>
                        登录
                      </Button>
                    )}
                  </div>
                </div>
              )}

              {/* 错误信息 */}
              {codexStatus.error && (
                <div className="flex items-start gap-2 p-3 text-sm text-red-500 bg-red-500/10 rounded-md">
                  <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                  <span className="whitespace-pre-wrap">
                    {codexStatus.error}
                  </span>
                </div>
              )}
            </div>
          )}

          {/* 说明 */}
          <div className="p-4 bg-muted/30 rounded-md space-y-2">
            <h4 className="text-sm font-medium">关于 Codex CLI</h4>
            <p className="text-xs text-muted-foreground">
              Codex CLI 是 OpenAI 提供的命令行工具，支持 Agent
              模式进行代码生成和工具调用。 它使用自己的认证系统（通过{" "}
              <code>codex login</code>）， 与 ProxyCast 凭证池中的 API Key
              是独立的。
            </p>
            <div className="flex gap-2 mt-2">
              <Button
                variant="ghost"
                size="sm"
                className="h-auto p-0 text-xs text-primary hover:underline"
                onClick={() =>
                  window.open("https://github.com/openai/codex", "_blank")
                }
              >
                <ExternalLink className="w-3 h-3 mr-1" />
                GitHub 文档
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 说明卡片 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">CLI 工具 vs API 凭证</CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground space-y-2">
          <p>
            <strong>CLI 工具</strong>（如 Codex CLI）有自己的认证系统，
            通过命令行登录后可以在 Agent 模式中使用。
          </p>
          <p>
            <strong>API 凭证</strong>（在凭证池中管理）用于 ProxyCast
            代理服务器，将请求转发到各个 AI 服务。
          </p>
          <p>两者是独立的，可以同时使用不同的账号。</p>
        </CardContent>
      </Card>
    </div>
  );
}
