/**
 * 连接管理设置组件
 *
 * 提供 Waveterm 风格的 connections.json 编辑界面。
 * 支持直接编辑 JSON 配置文件。
 *
 * @module components/settings/ConnectionsSettings
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
  Save,
  RefreshCw,
  FolderOpen,
  AlertCircle,
  CheckCircle,
} from "lucide-react";
import {
  getConnectionConfigPath,
  getRawConnectionConfig,
  saveRawConnectionConfig,
} from "@/lib/connection-api";

export function ConnectionsSettings() {
  const [configPath, setConfigPath] = useState<string>("");
  const [configContent, setConfigContent] = useState<string>("");
  const [originalContent, setOriginalContent] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // 加载配置
  const loadConfig = useCallback(async () => {
    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      const [path, content] = await Promise.all([
        getConnectionConfigPath(),
        getRawConnectionConfig(),
      ]);
      setConfigPath(path);
      setConfigContent(content);
      setOriginalContent(content);
    } catch (err) {
      console.error("[ConnectionsSettings] 加载配置失败:", err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  // 首次加载
  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // 保存配置
  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSuccess(null);

    try {
      // 先验证 JSON 格式
      JSON.parse(configContent);

      const result = await saveRawConnectionConfig(configContent);
      if (result.success) {
        setOriginalContent(configContent);
        setSuccess("配置已保存");
        setTimeout(() => setSuccess(null), 3000);
      } else {
        setError(result.error || "保存失败");
      }
    } catch (err) {
      if (err instanceof SyntaxError) {
        setError(`JSON 格式错误: ${err.message}`);
      } else {
        setError(String(err));
      }
    } finally {
      setSaving(false);
    }
  };

  // 检查是否有未保存的修改
  const hasChanges = configContent !== originalContent;

  // 格式化 JSON
  const handleFormat = () => {
    try {
      const parsed = JSON.parse(configContent);
      setConfigContent(JSON.stringify(parsed, null, 2));
    } catch {
      setError("JSON 格式错误，无法格式化");
    }
  };

  return (
    <div className="space-y-6 max-w-4xl">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <span>连接配置</span>
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={loadConfig}
                disabled={loading}
              >
                <RefreshCw
                  className={`w-4 h-4 mr-2 ${loading ? "animate-spin" : ""}`}
                />
                刷新
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={handleFormat}
                disabled={loading}
              >
                格式化
              </Button>
              <Button
                size="sm"
                onClick={handleSave}
                disabled={saving || !hasChanges}
              >
                <Save className="w-4 h-4 mr-2" />
                {saving ? "保存中..." : "保存"}
              </Button>
            </div>
          </CardTitle>
          <CardDescription className="flex items-center gap-2">
            <FolderOpen className="w-4 h-4" />
            <code className="text-xs bg-muted px-2 py-1 rounded">
              {configPath}
            </code>
          </CardDescription>
        </CardHeader>
        <CardContent>
          {/* 状态提示 */}
          {error && (
            <div className="flex items-center gap-2 p-3 mb-4 text-sm text-red-500 bg-red-500/10 rounded-md">
              <AlertCircle className="w-4 h-4" />
              {error}
            </div>
          )}
          {success && (
            <div className="flex items-center gap-2 p-3 mb-4 text-sm text-green-500 bg-green-500/10 rounded-md">
              <CheckCircle className="w-4 h-4" />
              {success}
            </div>
          )}

          {/* JSON 编辑器 */}
          <div className="relative">
            <textarea
              className="w-full h-[400px] p-4 font-mono text-sm bg-zinc-900 text-zinc-100 border border-zinc-700 rounded-md resize-none focus:outline-none focus:ring-2 focus:ring-primary"
              value={configContent}
              onChange={(e) => setConfigContent(e.target.value)}
              placeholder={loading ? "加载中..." : "// 在此编辑连接配置"}
              disabled={loading}
              spellCheck={false}
            />
            {hasChanges && (
              <div className="absolute top-2 right-2 px-2 py-1 text-xs bg-yellow-500/20 text-yellow-500 rounded">
                未保存
              </div>
            )}
          </div>

          {/* 帮助信息 */}
          <div className="mt-4 p-4 bg-muted/50 rounded-md">
            <h4 className="font-medium mb-2">配置格式说明</h4>
            <pre className="text-xs text-muted-foreground overflow-x-auto">
              {`{
  "connections": {
    "my-server": {
      "type": "ssh",
      "user": "root",
      "host": "192.168.1.100",
      "port": 22,
      "identityFile": "~/.ssh/id_rsa"
    },
    "dev-server": {
      "type": "ssh",
      "user": "developer",
      "host": "dev.example.com"
    }
  }
}`}
            </pre>
            <p className="mt-2 text-xs text-muted-foreground">
              支持的字段: <code>type</code> (ssh/local/wsl), <code>user</code>,{" "}
              <code>host</code>, <code>port</code>, <code>identityFile</code>,{" "}
              <code>proxyJump</code>
            </p>
          </div>
        </CardContent>
      </Card>

      {/* SSH 配置提示 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">SSH 配置</CardTitle>
          <CardDescription>
            系统 SSH 配置文件 (~/.ssh/config) 中的 Host 也会自动显示在连接列表中
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            你可以在 ~/.ssh/config 中配置 SSH
            连接，这些配置会自动被识别并显示在终端连接选择器中。
            如果需要自定义配置或添加不在 SSH 配置中的连接，请使用上方的 JSON
            编辑器。
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
