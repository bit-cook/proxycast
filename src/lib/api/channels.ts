/**
 * 渠道管理 API
 *
 * 封装所有渠道管理的 API 调用，包括 AI 渠道和通知渠道
 */

import { safeInvoke } from "@/lib/dev-bridge";

// ============================================================================
// 类型定义
// ============================================================================

/**
 * AI 提供商引擎类型
 */
export enum AIProviderEngine {
  OPENAI = "openai",
  OLLAMA = "ollama",
  ANTHROPIC = "anthropic",
}

/**
 * 模型信息
 */
export interface ModelInfo {
  id: string;
  name: string;
  description?: string;
}

/**
 * AI 渠道配置
 */
export interface AIChannelConfig {
  name: string;
  engine: AIProviderEngine;
  display_name: string;
  description?: string;
  api_key_env: string;
  base_url: string;
  models: ModelInfo[];
  headers?: Record<string, string>;
  timeout_seconds?: number;
  supports_streaming?: boolean;
}

/**
 * AI 渠道
 */
export interface AIChannel {
  id: string;
  name: string;
  engine: AIProviderEngine;
  display_name: string;
  description?: string;
  api_key_env: string;
  base_url: string;
  models: ModelInfo[];
  headers?: Record<string, string>;
  timeout_seconds?: number;
  supports_streaming?: boolean;
  enabled: boolean;
}

/**
 * 连接测试结果
 */
export interface ConnectionTestResult {
  success: boolean;
  message: string;
  details?: unknown;
}

/**
 * 通知渠道类型
 */
export enum NotificationChannelType {
  FEISHU = "feishu",
  TELEGRAM = "telegram",
  DISCORD = "discord",
}

/**
 * 飞书配置
 */
export interface FeishuConfig {
  webhook_url: string;
  secret?: string;
}

/**
 * Telegram 配置
 */
export interface TelegramConfig {
  bot_token: string;
  chat_id: string;
}

/**
 * Discord 配置
 */
export interface DiscordConfig {
  webhook_url: string;
  username?: string;
}

/**
 * 通知渠道特定配置
 */
export type NotificationChannelSpecificConfig =
  | { type: NotificationChannelType.FEISHU } & FeishuConfig
  | { type: NotificationChannelType.TELEGRAM } & TelegramConfig
  | { type: NotificationChannelType.DISCORD } & DiscordConfig;

/**
 * 通知渠道配置
 */
export interface NotificationChannelConfig {
  name: string;
  channel_type: NotificationChannelType;
  config: NotificationChannelSpecificConfig;
}

/**
 * 通知渠道
 */
export interface NotificationChannel {
  id: string;
  name: string;
  channel_type: NotificationChannelType;
  config: NotificationChannelSpecificConfig;
  enabled: boolean;
}

/**
 * 测试消息结果
 */
export interface TestMessageResult {
  success: boolean;
  message: string;
}

// ============================================================================
// AI 渠道 API
// ============================================================================

/**
 * AI 渠道 API
 */
export const aiChannelsApi = {
  /**
   * 获取所有 AI 渠道
   */
  async getChannels(): Promise<AIChannel[]> {
    return safeInvoke<Array<unknown>>("get_ai_channels").then((channels) =>
      channels.map(mapAIChannelFromBackend),
    );
  },

  /**
   * 获取单个 AI 渠道
   */
  async getChannel(id: string): Promise<AIChannel> {
    const channel = await safeInvoke<AIChannel>("get_ai_channel", { id });
    return mapAIChannelFromBackend(channel);
  },

  /**
   * 创建 AI 渠道
   */
  async createChannel(config: AIChannelConfig): Promise<AIChannel> {
    const channel = await safeInvoke<AIChannel>("create_ai_channel", {
      config: mapAIChannelConfigToBackend(config),
    });
    return mapAIChannelFromBackend(channel);
  },

  /**
   * 更新 AI 渠道
   */
  async updateChannel(
    id: string,
    config: Partial<AIChannelConfig>,
  ): Promise<AIChannel> {
    const channel = await safeInvoke<AIChannel>("update_ai_channel", {
      id,
      config: mapAIChannelConfigToBackend(
        config as AIChannelConfig,
      ),
    });
    return mapAIChannelFromBackend(channel);
  },

  /**
   * 删除 AI 渠道
   */
  async deleteChannel(id: string): Promise<void> {
    await safeInvoke<void>("delete_ai_channel", { id });
  },

  /**
   * 测试 AI 渠道连接
   */
  async testChannel(id: string): Promise<ConnectionTestResult> {
    return safeInvoke<ConnectionTestResult>("test_ai_channel", { id });
  },
};

// ============================================================================
// 通知渠道 API
// ============================================================================

/**
 * 通知渠道 API
 */
export const notificationChannelsApi = {
  /**
   * 获取所有通知渠道
   */
  async getChannels(): Promise<NotificationChannel[]> {
    return safeInvoke<Array<unknown>>("get_notification_channels").then(
      (channels) => channels.map(mapNotificationChannelFromBackend),
    );
  },

  /**
   * 获取单个通知渠道
   */
  async getChannel(id: string): Promise<NotificationChannel> {
    const channel = await safeInvoke<NotificationChannel>(
      "get_notification_channel",
      { id },
    );
    return mapNotificationChannelFromBackend(channel);
  },

  /**
   * 创建通知渠道
   */
  async createChannel(
    config: NotificationChannelConfig,
  ): Promise<NotificationChannel> {
    const channel = await safeInvoke<NotificationChannel>(
      "create_notification_channel",
      { config: mapNotificationChannelConfigToBackend(config) },
    );
    return mapNotificationChannelFromBackend(channel);
  },

  /**
   * 更新通知渠道
   */
  async updateChannel(
    id: string,
    config: Partial<NotificationChannelConfig>,
  ): Promise<NotificationChannel> {
    const channel = await safeInvoke<NotificationChannel>(
      "update_notification_channel",
      {
        id,
        config: mapNotificationChannelConfigToBackend(
          config as NotificationChannelConfig,
        ),
      },
    );
    return mapNotificationChannelFromBackend(channel);
  },

  /**
   * 删除通知渠道
   */
  async deleteChannel(id: string): Promise<void> {
    await safeInvoke<void>("delete_notification_channel", { id });
  },

  /**
   * 发送测试消息到通知渠道
   */
  async testChannel(id: string, message: string): Promise<TestMessageResult> {
    return safeInvoke<TestMessageResult>("test_notification_channel", {
      id,
      message,
    });
  },
};

// ============================================================================
// 数据映射函数
// ============================================================================

/**
 * 将后端 AI 渠道数据映射到前端格式
 */
function mapAIChannelFromBackend(channel: unknown): AIChannel {
  const data = channel as Record<string, unknown>;
  return {
    id: data.id as string,
    name: data.name as string,
    engine: data.engine as AIProviderEngine,
    display_name: data.display_name as string,
    description: data.description as string | undefined,
    api_key_env: data.api_key_env as string,
    base_url: data.base_url as string,
    models: (data.models as Array<Record<string, unknown>> | undefined)?.map(
      (m) => ({
        id: m.id as string,
        name: m.name as string,
        description: m.description as string | undefined,
      }),
    ) ?? [],
    headers: data.headers as Record<string, string> | undefined,
    timeout_seconds: data.timeout_seconds as number | undefined,
    supports_streaming: data.supports_streaming as boolean | undefined,
    enabled: data.enabled as boolean,
  };
}

/**
 * 将前端 AI 渠道配置映射到后端格式
 */
function mapAIChannelConfigToBackend(config: AIChannelConfig): Record<string, unknown> {
  return {
    name: config.name,
    engine: config.engine,
    display_name: config.display_name,
    description: config.description,
    api_key_env: config.api_key_env,
    base_url: config.base_url,
    models: config.models,
    headers: config.headers,
    timeout_seconds: config.timeout_seconds,
    supports_streaming: config.supports_streaming,
  };
}

/**
 * 将后端通知渠道数据映射到前端格式
 */
function mapNotificationChannelFromBackend(channel: unknown): NotificationChannel {
  const data = channel as Record<string, unknown>;
  const baseConfig = {
    id: data.id as string,
    name: data.name as string,
    channel_type: data.channel_type as NotificationChannelType,
    enabled: data.enabled as boolean,
  };

  const config = data.config as Record<string, unknown>;

  switch (baseConfig.channel_type) {
    case NotificationChannelType.FEISHU:
      return {
        ...baseConfig,
        config: {
          type: NotificationChannelType.FEISHU,
          webhook_url: config.webhook_url as string,
          secret: config.secret as string | undefined,
        },
      };
    case NotificationChannelType.TELEGRAM:
      return {
        ...baseConfig,
        config: {
          type: NotificationChannelType.TELEGRAM,
          bot_token: config.bot_token as string,
          chat_id: config.chat_id as string,
        },
      };
    case NotificationChannelType.DISCORD:
      return {
        ...baseConfig,
        config: {
          type: NotificationChannelType.DISCORD,
          webhook_url: config.webhook_url as string,
          username: config.username as string | undefined,
        },
      };
    default:
      throw new Error(`未知的通知渠道类型: ${baseConfig.channel_type}`);
  }
}

/**
 * 将前端通知渠道配置映射到后端格式
 */
function mapNotificationChannelConfigToBackend(
  config: NotificationChannelConfig,
): Record<string, unknown> {
  return {
    name: config.name,
    channel_type: config.channel_type,
    config: config.config,
  };
}
