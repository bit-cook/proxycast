/**
 * 渠道管理设置页面
 *
 * 提供两类渠道的管理：
 * - AI 模型提供商渠道（OpenAI、Ollama、Anthropic 等）
 * - 消息通知渠道（飞书、Telegram、Discord 等）
 */

import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { SettingHeader } from "@/components/settings-v2/features/SettingHeader";
import { AIChannelsList } from "./AIChannelsList";
import { NotificationChannelsList } from "./NotificationChannelsList";

export interface ChannelsSettingsProps {
  className?: string;
}

export function ChannelsSettings({ className }: ChannelsSettingsProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"ai" | "notification">("ai");

  return (
    <div className={className}>
      <SettingHeader title={t("渠道管理", "渠道管理")} />

      <Tabs
        value={activeTab}
        onValueChange={(value) => setActiveTab(value as "ai" | "notification")}
        className="w-full"
      >
        <TabsList className="grid w-full max-w-md grid-cols-2">
          <TabsTrigger value="ai">
            {t("AI 模型提供商", "AI 模型提供商")}
          </TabsTrigger>
          <TabsTrigger value="notification">
            {t("消息通知渠道", "消息通知渠道")}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="ai" className="mt-6">
          <AIChannelsList />
        </TabsContent>

        <TabsContent value="notification" className="mt-6">
          <NotificationChannelsList />
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default ChannelsSettings;
