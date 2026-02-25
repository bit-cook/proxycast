/**
 * AI 渠道列表组件
 *
 * 显示 AI 模型提供商渠道列表，支持添加、编辑、删除、启用/禁用和连接测试
 */

import React, { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Pencil, Trash2, Power } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableHeader,
  TableBody,
  TableHead,
  TableRow,
  TableCell,
} from "@/components/ui/table";
import {
  aiChannelsApi,
  type AIChannel,
  type AIChannelConfig,
  AIProviderEngine,
} from "@/lib/api/channels";
import { AIChannelFormModal } from "./AIChannelFormModal";
import { ConnectionTestButton } from "./ConnectionTestButton";
import { DeleteChannelDialog } from "./DeleteChannelDialog";

export function AIChannelsList() {
  const { t } = useTranslation();
  const [channels, setChannels] = useState<AIChannel[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddModal, setShowAddModal] = useState(false);
  const [editingChannel, setEditingChannel] = useState<AIChannel | null>(null);
  const [deletingChannel, setDeletingChannel] = useState<AIChannel | null>(null);

  // 加载渠道列表
  const loadChannels = useCallback(async () => {
    setLoading(true);
    try {
      const data = await aiChannelsApi.getChannels();
      setChannels(data);
    } catch (e) {
      console.error("加载 AI 渠道失败:", e);
      toast.error(`加载失败: ${e instanceof Error ? e.message : e}`);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadChannels();
  }, [loadChannels]);

  // 添加渠道
  const handleAdd = useCallback(
    async (config: AIChannelConfig) => {
      try {
        await aiChannelsApi.createChannel(config);
        toast.success("AI 渠道已创建");
        await loadChannels();
        setShowAddModal(false);
      } catch (e) {
        throw e;
      }
    },
    [loadChannels],
  );

  // 更新渠道
  const handleUpdate = useCallback(
    async (id: string, config: AIChannelConfig) => {
      try {
        await aiChannelsApi.updateChannel(id, config);
        toast.success("AI 渠道已更新");
        await loadChannels();
        setEditingChannel(null);
      } catch (e) {
        throw e;
      }
    },
    [loadChannels],
  );

  // 删除渠道
  const handleDelete = useCallback(async () => {
    if (!deletingChannel) return;

    try {
      await aiChannelsApi.deleteChannel(deletingChannel.id);
      toast.success("AI 渠道已删除");
      await loadChannels();
      setDeletingChannel(null);
    } catch (e) {
      toast.error(`删除失败: ${e instanceof Error ? e.message : e}`);
    }
  }, [deletingChannel, loadChannels]);

  // 切换启用状态
  const handleToggle = useCallback(
    async (channel: AIChannel, enabled: boolean) => {
      try {
        await aiChannelsApi.updateChannel(channel.id, {
          ...channel,
          enabled,
        } as AIChannelConfig);
        toast.success(enabled ? "AI 渠道已启用" : "AI 渠道已禁用");
        await loadChannels();
      } catch (e) {
        toast.error(`操作失败: ${e instanceof Error ? e.message : e}`);
      }
    },
    [loadChannels],
  );

  // 获取引擎名称
  const getEngineName = (engine: AIProviderEngine): string => {
    const names: Record<AIProviderEngine, string> = {
      [AIProviderEngine.OPENAI]: "OpenAI",
      [AIProviderEngine.OLLAMA]: "Ollama",
      [AIProviderEngine.ANTHROPIC]: "Anthropic",
    };
    return names[engine] || engine;
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-32">
        <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* 头部操作栏 */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium">
            {t("AI 模型提供商", "AI 模型提供商")}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("管理 OpenAI、Ollama、Anthropic 等 AI 服务提供商", "管理 OpenAI、Ollama、Anthropic 等 AI 服务提供商")}
          </p>
        </div>
        <Button size="sm" onClick={() => setShowAddModal(true)}>
          <Plus className="h-4 w-4 mr-1" />
          {t("添加 AI 渠道", "添加 AI 渠道")}
        </Button>
      </div>

      {/* 渠道列表 */}
      {channels.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground text-sm">
          <p>{t("暂无 AI 渠道", "暂无 AI 渠道")}</p>
          <p className="text-xs mt-1">
            {t("点击上方按钮添加您的第一个 AI 渠道", "点击上方按钮添加您的第一个 AI 渠道")}
          </p>
        </div>
      ) : (
        <div className="border rounded-lg overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("名称", "名称")}</TableHead>
                <TableHead>{t("引擎", "引擎")}</TableHead>
                <TableHead>{t("API 地址", "API 地址")}</TableHead>
                <TableHead>{t("模型数", "模型数")}</TableHead>
                <TableHead>{t("状态", "状态")}</TableHead>
                <TableHead className="text-right">
                  {t("操作", "操作")}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {channels.map((channel) => (
                <TableRow key={channel.id}>
                  <TableCell className="font-medium">
                    {channel.display_name}
                  </TableCell>
                  <TableCell>{getEngineName(channel.engine)}</TableCell>
                  <TableCell className="text-muted-foreground text-xs">
                    {channel.base_url}
                  </TableCell>
                  <TableCell>{channel.models.length}</TableCell>
                  <TableCell>
                    <Switch
                      checked={channel.enabled}
                      onCheckedChange={(checked) =>
                        handleToggle(channel, checked)
                      }
                    />
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1">
                      <ConnectionTestButton
                        channelId={channel.id}
                        channelName={channel.display_name}
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => setEditingChannel(channel)}
                      >
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => setDeletingChannel(channel)}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {/* 添加模态框 */}
      <AIChannelFormModal
        isOpen={showAddModal}
        onClose={() => setShowAddModal(false)}
        onSubmit={handleAdd}
      />

      {/* 编辑模态框 */}
      {editingChannel && (
        <AIChannelFormModal
          isOpen={!!editingChannel}
          onClose={() => setEditingChannel(null)}
          onSubmit={(config) => handleUpdate(editingChannel.id, config)}
          initialData={editingChannel}
        />
      )}

      {/* 删除确认对话框 */}
      <DeleteChannelDialog
        isOpen={!!deletingChannel}
        channelName={deletingChannel?.display_name ?? ""}
        onClose={() => setDeletingChannel(null)}
        onConfirm={handleDelete}
      />
    </div>
  );
}

export default AIChannelsList;
