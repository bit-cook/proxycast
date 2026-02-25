/**
 * 连接测试按钮组件
 *
 * 用于测试 AI 渠道的连接状态
 */

import React, { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Plug, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { aiChannelsApi } from "@/lib/api/channels";

export interface ConnectionTestButtonProps {
  channelId: string;
  channelName: string;
}

export function ConnectionTestButton({
  channelId,
  channelName,
}: ConnectionTestButtonProps) {
  const { t } = useTranslation();
  const [testing, setTesting] = useState(false);

  const handleTest = useCallback(async () => {
    setTesting(true);
    try {
      const result = await aiChannelsApi.testChannel(channelId);
      if (result.success) {
        toast.success(t("连接成功", "连接成功") + ": " + result.message);
      } else {
        toast.error(t("连接失败", "连接失败") + ": " + result.message);
      }
    } catch (e) {
      toast.error(
        t("测试失败", "测试失败") + ": " + (e instanceof Error ? e.message : e),
      );
    } finally {
      setTesting(false);
    }
  }, [channelId, t]);

  return (
    <Button
      variant="ghost"
      size="icon"
      className="h-8 w-8"
      onClick={handleTest}
      disabled={testing}
      title={t("测试连接", "测试连接")}
    >
      {testing ? (
        <Loader2 className="h-4 w-4 animate-spin" />
      ) : (
        <Plug className="h-4 w-4" />
      )}
    </Button>
  );
}

export default ConnectionTestButton;
