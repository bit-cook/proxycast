/**
 * 通知渠道列表组件
 *
 * 显示消息通知渠道列表（飞书、Telegram、Discord），支持添加、编辑、删除、启用/禁用和发送测试消息
 */

import React, { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Pencil, Trash2, Send } from "lucide-react";
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
  notificationChannelsApi,
  type NotificationChannel,
  type NotificationChannelConfig,
  NotificationChannelType,
} from "@/lib/api/channels";
import { NotificationChannelFormModal } from "./NotificationChannelFormModal";
import { SendTestMessageButton } from "./SendTestMessageButton";
import { DeleteChannelDialog } from "./DeleteChannelDialog";

export function NotificationChannelsList() {
  const { t } = useTranslation();
  const [channels, setChannels] = useState<NotificationChannel[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddModal, setShowAddModal] = useState(false);
  const [editingChannel, setEditingChannel] = useState<NotificationChannel | null>(null);
  const [deletingChannel, setDeletingChannel] = useState<NotificationChannel | null>(null);

  // 加载渠道列表
  const loadChannels = useCallback(async () => {
    setLoading(true);
    try {
      const data = await notificationChannelsApi.getChannels();
      setChannels(data);
    } catch (e) {
      console.error("加载通知渠道失败:", e);
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
    async (config: NotificationChannelConfig) => {
      try {
        await notificationChannelsApi.createChannel(config);
        toast.success("通知渠道已创建");
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
    async (id: string, config: NotificationChannelConfig) => {
      try {
        await notificationChannelsApi.updateChannel(id, config);
        toast.success("通知渠道已更新");
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
      await notificationChannelsApi.deleteChannel(deletingChannel.id);
      toast.success("通知渠道已删除");
      await loadChannels();
      setDeletingChannel(null);
    } catch (e) {
      toast.error(`删除失败: ${e instanceof Error ? e.message : e}`);
    }
  }, [deletingChannel, loadChannels]);

  // 切换启用状态
  const handleToggle = useCallback(
    async (channel: NotificationChannel, enabled: boolean) => {
      try {
        await notificationChannelsApi.updateChannel(channel.id, {
          ...channel,
          enabled,
        } as NotificationChannelConfig);
        toast.success(enabled ? "通知渠道已启用" : "通知渠道已禁用");
        await loadChannels();
      } catch (e) {
        toast.error(`操作失败: ${e instanceof Error ? e.message : e}`);
      }
    },
    [loadChannels],
  );

  // 获取渠道类型名称
  const getChannelTypeName = (type: NotificationChannelType): string => {
    const names: Record<NotificationChannelType, string> = {
      [NotificationChannelType.FEISHU]: t("飞书", "飞书"),
      [NotificationChannelType.TELEGRAM]: t("Telegram", "Telegram"),
      [NotificationChannelType.DISCORD]: t("Discord", "Discord"),
    };
    return names[type] || type;
  };

  // 获取渠道配置摘要
  const getConfigSummary = (channel: NotificationChannel): string => {
    switch (channel.channel_type) {
      case NotificationChannelType.FEISHU:
        return t("Webhook", "Webhook");
      case NotificationChannelType.TELEGRAM:
        return t("Bot", "Bot");
      case NotificationChannelType.DISCORD:
        return t("Webhook", "Webhook");
      default:
        return "-";
    }
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
            {t("消息通知渠道", "消息通知渠道")}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("管理飞书、Telegram、Discord 等消息通知渠道", "管理飞书、Telegram、Discord 等消息通知渠道")}
          </p>
        </div>
        <Button size="sm" onClick={() => setShowAddModal(true)}>
          <Plus className="h-4 w-4 mr-1" />
          {t("添加通知渠道", "添加通知渠道")}
        </Button>
      </div>

      {/* 渠道列表 */}
      {channels.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground text-sm">
          <p>{t("暂无通知渠道", "暂无通知渠道")}</p>
          <p className="text-xs mt-1">
            {t("点击上方按钮添加您的第一个通知渠道", "点击上方按钮添加您的第一个通知渠道")}
          </p>
        </div>
      ) : (
        <div className="border rounded-lg overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("名称", "名称")}</TableHead>
                <TableHead>{t("类型", "类型")}</TableHead>
                <TableHead>{t("配置", "配置")}</TableHead>
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
                    {channel.name}
                  </TableCell>
                  <TableCell>
                    <span className="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-primary/10 text-primary">
                      {getChannelTypeName(channel.channel_type)}
                    </span>
                  </TableCell>
                  <TableCell className="text-muted-foreground text-xs">
                    {getConfigSummary(channel)}
                  </TableCell>
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
                      <SendTestMessageButton
                        channelId={channel.id}
                        channelName={channel.name}
                        channelType={channel.channel_type}
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
      <NotificationChannelFormModal
        isOpen={showAddModal}
        onClose={() => setShowAddModal(false)}
        onSubmit={handleAdd}
      />

      {/* 编辑模态框 */}
      {editingChannel && (
        <NotificationChannelFormModal
          isOpen={!!editingChannel}
          onClose={() => setEditingChannel(null)}
          onSubmit={(config) => handleUpdate(editingChannel.id, config)}
          initialData={editingChannel}
        />
      )}

      {/* 删除确认对话框 */}
      <DeleteChannelDialog
        isOpen={!!deletingChannel}
        channelName={deletingChannel?.name ?? ""}
        onClose={() => setDeletingChannel(null)}
        onConfirm={handleDelete}
      />
    </div>
  );
}

export default NotificationChannelsList;
