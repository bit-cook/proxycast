/**
 * 通知渠道表单模态框组件
 *
 * 用于添加或编辑消息通知渠道配置（飞书、Telegram、Discord）
 */

import React, { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Modal, ModalHeader, ModalBody, ModalFooter } from "@/components/Modal";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  type NotificationChannel,
  type NotificationChannelConfig,
  NotificationChannelType,
} from "@/lib/api/channels";

export interface NotificationChannelFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (config: NotificationChannelConfig) => Promise<void>;
  initialData?: NotificationChannel;
}

interface FormState {
  channel_type: NotificationChannelType;
  name: string;
  // 飞书配置
  feishu_webhook_url: string;
  feishu_secret: string;
  // Telegram 配置
  telegram_bot_token: string;
  telegram_chat_id: string;
  // Discord 配置
  discord_webhook_url: string;
  discord_username: string;
}

const INITIAL_FORM_STATE: FormState = {
  channel_type: NotificationChannelType.FEISHU,
  name: "",
  feishu_webhook_url: "",
  feishu_secret: "",
  telegram_bot_token: "",
  telegram_chat_id: "",
  discord_webhook_url: "",
  discord_username: "",
};

export function NotificationChannelFormModal({
  isOpen,
  onClose,
  onSubmit,
  initialData,
}: NotificationChannelFormModalProps) {
  const { t } = useTranslation();
  const [formState, setFormState] = useState<FormState>(INITIAL_FORM_STATE);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // 重置表单或填充初始数据
  useEffect(() => {
    if (isOpen) {
      if (initialData) {
        const newState = { ...INITIAL_FORM_STATE };
        newState.channel_type = initialData.channel_type;
        newState.name = initialData.name;

        // 根据渠道类型填充特定配置
        switch (initialData.channel_type) {
          case NotificationChannelType.FEISHU: {
            const cfg = initialData.config as { type: NotificationChannelType.FEISHU; webhook_url: string; secret?: string };
            newState.feishu_webhook_url = cfg.webhook_url ?? "";
            newState.feishu_secret = cfg.secret ?? "";
            break;
          }
          case NotificationChannelType.TELEGRAM: {
            const cfg = initialData.config as { type: NotificationChannelType.TELEGRAM; bot_token: string; chat_id: string };
            newState.telegram_bot_token = cfg.bot_token;
            newState.telegram_chat_id = cfg.chat_id;
            break;
          }
          case NotificationChannelType.DISCORD: {
            const cfg = initialData.config as { type: NotificationChannelType.DISCORD; webhook_url: string; username?: string };
            newState.discord_webhook_url = cfg.webhook_url;
            newState.discord_username = cfg.username ?? "";
            break;
          }
        }

        setFormState(newState);
      } else {
        setFormState(INITIAL_FORM_STATE);
      }
      setErrors({});
    }
  }, [isOpen, initialData]);

  const updateField = useCallback(
    <K extends keyof FormState>(field: K, value: FormState[K]) => {
      setFormState((prev) => ({ ...prev, [field]: value }));
      // 清除该字段的错误
      const fieldKey = field as string;
      if (errors[fieldKey]) {
        setErrors((prev) => {
          const newErrors = { ...prev };
          delete newErrors[fieldKey];
          return newErrors;
        });
      }
    },
    [errors],
  );

  const validateForm = useCallback((): boolean => {
    const newErrors: Record<string, string> = {};

    if (!formState.name.trim()) {
      newErrors.name = t("名称不能为空", "名称不能为空");
    }

    // 根据渠道类型验证特定字段
    switch (formState.channel_type) {
      case NotificationChannelType.FEISHU:
        if (!formState.feishu_webhook_url.trim()) {
          newErrors.feishu_webhook_url = t("Webhook 地址不能为空", "Webhook 地址不能为空");
        } else {
          try {
            new URL(formState.feishu_webhook_url.trim());
          } catch {
            newErrors.feishu_webhook_url = t("请输入有效的 URL", "请输入有效的 URL");
          }
        }
        break;

      case NotificationChannelType.TELEGRAM:
        if (!formState.telegram_bot_token.trim()) {
          newErrors.telegram_bot_token = t("Bot Token 不能为空", "Bot Token 不能为空");
        }
        if (!formState.telegram_chat_id.trim()) {
          newErrors.telegram_chat_id = t("聊天 ID 不能为空", "聊天 ID 不能为空");
        }
        break;

      case NotificationChannelType.DISCORD:
        if (!formState.discord_webhook_url.trim()) {
          newErrors.discord_webhook_url = t("Webhook URL 不能为空", "Webhook URL 不能为空");
        } else {
          try {
            new URL(formState.discord_webhook_url.trim());
          } catch {
            newErrors.discord_webhook_url = t("请输入有效的 URL", "请输入有效的 URL");
          }
        }
        break;
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [formState, t]);

  const handleSubmit = useCallback(async () => {
    if (!validateForm()) {
      return;
    }

    setIsSubmitting(true);
    try {
      let config: NotificationChannelConfig;

      switch (formState.channel_type) {
        case NotificationChannelType.FEISHU:
          config = {
            name: formState.name.trim(),
            channel_type: formState.channel_type,
            config: {
              type: NotificationChannelType.FEISHU,
              webhook_url: formState.feishu_webhook_url.trim(),
              secret: formState.feishu_secret.trim() || undefined,
            },
          };
          break;

        case NotificationChannelType.TELEGRAM:
          config = {
            name: formState.name.trim(),
            channel_type: formState.channel_type,
            config: {
              type: NotificationChannelType.TELEGRAM,
              bot_token: formState.telegram_bot_token.trim(),
              chat_id: formState.telegram_chat_id.trim(),
            },
          };
          break;

        case NotificationChannelType.DISCORD:
          config = {
            name: formState.name.trim(),
            channel_type: formState.channel_type,
            config: {
              type: NotificationChannelType.DISCORD,
              webhook_url: formState.discord_webhook_url.trim(),
              username: formState.discord_username.trim() || undefined,
            },
          };
          break;

        default:
          throw new Error("未知的通知渠道类型");
      }

      await onSubmit(config);
    } catch (e) {
      console.error("提交失败:", e);
      // 错误已经在调用处处理
    } finally {
      setIsSubmitting(false);
    }
  }, [formState, onSubmit, validateForm]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} maxWidth="max-w-md">
      <ModalHeader>
        {initialData
          ? t("编辑通知渠道", "编辑通知渠道")
          : t("添加通知渠道", "添加通知渠道")}
      </ModalHeader>

      <ModalBody className="space-y-4">
        {/* 渠道类型 */}
        <div className="space-y-1.5">
          <Label htmlFor="channel_type">
            {t("渠道类型", "渠道类型")} <span className="text-red-500">*</span>
          </Label>
          <Select
            value={formState.channel_type}
            onValueChange={(value) =>
              updateField("channel_type", value as NotificationChannelType)
            }
            disabled={isSubmitting || !!initialData}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={NotificationChannelType.FEISHU}>
                {t("飞书", "飞书")}
              </SelectItem>
              <SelectItem value={NotificationChannelType.TELEGRAM}>
                Telegram
              </SelectItem>
              <SelectItem value={NotificationChannelType.DISCORD}>
                Discord
              </SelectItem>
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            {t("选择消息通知渠道类型", "选择消息通知渠道类型")}
          </p>
        </div>

        {/* 名称 */}
        <div className="space-y-1.5">
          <Label htmlFor="name">
            {t("名称", "名称")} <span className="text-red-500">*</span>
          </Label>
          <Input
            id="name"
            type="text"
            value={formState.name}
            onChange={(e) => updateField("name", e.target.value)}
            placeholder={t("我的飞书通知", "我的飞书通知")}
            disabled={isSubmitting}
            className={errors.name ? "border-red-500" : ""}
          />
          {errors.name && <p className="text-xs text-red-500">{errors.name}</p>}
        </div>

        {/* 飞书配置 */}
        {formState.channel_type === NotificationChannelType.FEISHU && (
          <>
            <div className="border-t pt-4">
              <h4 className="text-sm font-medium mb-3">
                {t("飞书配置", "飞书配置")}
              </h4>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="feishu_webhook_url">
                Webhook {t("地址", "地址")} <span className="text-red-500">*</span>
              </Label>
              <Input
                id="feishu_webhook_url"
                type="text"
                value={formState.feishu_webhook_url}
                onChange={(e) => updateField("feishu_webhook_url", e.target.value)}
                placeholder="https://open.feishu.cn/open-apis/bot/v2/hook/..."
                disabled={isSubmitting}
                className={errors.feishu_webhook_url ? "border-red-500" : ""}
              />
              {errors.feishu_webhook_url && (
                <p className="text-xs text-red-500">{errors.feishu_webhook_url}</p>
              )}
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="feishu_secret">
                {t("签名密钥", "签名密钥")} ({t("可选", "可选")})
              </Label>
              <Input
                id="feishu_secret"
                type="password"
                value={formState.feishu_secret}
                onChange={(e) => updateField("feishu_secret", e.target.value)}
                placeholder="********"
                disabled={isSubmitting}
              />
              <p className="text-xs text-muted-foreground">
                {t(
                  "用于验证 Webhook 请求的签名密钥",
                  "用于验证 Webhook 请求的签名密钥",
                )}
              </p>
            </div>
          </>
        )}

        {/* Telegram 配置 */}
        {formState.channel_type === NotificationChannelType.TELEGRAM && (
          <>
            <div className="border-t pt-4">
              <h4 className="text-sm font-medium mb-3">
                Telegram {t("配置", "配置")}
              </h4>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="telegram_bot_token">
                Bot Token <span className="text-red-500">*</span>
              </Label>
              <Input
                id="telegram_bot_token"
                type="password"
                value={formState.telegram_bot_token}
                onChange={(e) => updateField("telegram_bot_token", e.target.value)}
                placeholder="123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"
                disabled={isSubmitting}
                className={errors.telegram_bot_token ? "border-red-500" : ""}
              />
              {errors.telegram_bot_token && (
                <p className="text-xs text-red-500">{errors.telegram_bot_token}</p>
              )}
              <p className="text-xs text-muted-foreground">
                {t("从 @BotFather 获取", "从 @BotFather 获取")}
              </p>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="telegram_chat_id">
                Chat ID <span className="text-red-500">*</span>
              </Label>
              <Input
                id="telegram_chat_id"
                type="text"
                value={formState.telegram_chat_id}
                onChange={(e) => updateField("telegram_chat_id", e.target.value)}
                placeholder="-1001234567890"
                disabled={isSubmitting}
                className={errors.telegram_chat_id ? "border-red-500" : ""}
              />
              {errors.telegram_chat_id && (
                <p className="text-xs text-red-500">{errors.telegram_chat_id}</p>
              )}
              <p className="text-xs text-muted-foreground">
                {t("目标聊天或频道的 ID", "目标聊天或频道的 ID")}
              </p>
            </div>
          </>
        )}

        {/* Discord 配置 */}
        {formState.channel_type === NotificationChannelType.DISCORD && (
          <>
            <div className="border-t pt-4">
              <h4 className="text-sm font-medium mb-3">
                Discord {t("配置", "配置")}
              </h4>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="discord_webhook_url">
                Webhook URL <span className="text-red-500">*</span>
              </Label>
              <Input
                id="discord_webhook_url"
                type="text"
                value={formState.discord_webhook_url}
                onChange={(e) => updateField("discord_webhook_url", e.target.value)}
                placeholder="https://discord.com/api/webhooks/..."
                disabled={isSubmitting}
                className={errors.discord_webhook_url ? "border-red-500" : ""}
              />
              {errors.discord_webhook_url && (
                <p className="text-xs text-red-500">{errors.discord_webhook_url}</p>
              )}
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="discord_username">
                {t("自定义用户名", "自定义用户名")} ({t("可选", "可选")})
              </Label>
              <Input
                id="discord_username"
                type="text"
                value={formState.discord_username}
                onChange={(e) => updateField("discord_username", e.target.value)}
                placeholder="ProxyCast Bot"
                disabled={isSubmitting}
              />
              <p className="text-xs text-muted-foreground">
                {t("覆盖机器人的默认用户名", "覆盖机器人的默认用户名")}
              </p>
            </div>
          </>
        )}
      </ModalBody>

      <ModalFooter>
        <Button variant="outline" onClick={onClose} disabled={isSubmitting}>
          {t("取消", "取消")}
        </Button>
        <Button onClick={handleSubmit} disabled={isSubmitting}>
          {isSubmitting
            ? t("保存中...", "保存中...")
            : initialData
            ? t("保存", "保存")
            : t("添加", "添加")}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

export default NotificationChannelFormModal;
