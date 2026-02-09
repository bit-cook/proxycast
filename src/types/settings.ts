/**
 * 设置页面类型定义
 *
 * 定义设置分组和标签页的枚举
 */

/**
 * 设置分组 Key
 */
export enum SettingsGroupKey {
  Account = "account",
  General = "general",
  Agent = "agent",
  System = "system",
}

/**
 * 设置标签页
 */
export enum SettingsTabs {
  // 账号
  Profile = "profile",
  Stats = "stats",

  // 通用
  Appearance = "appearance",
  ChatAppearance = "chat-appearance",
  Hotkeys = "hotkeys",

  // 智能体
  Providers = "providers",
  Assistant = "assistant",
  Skills = "skills",
  Memory = "memory",
  ImageGen = "image-gen",
  Voice = "voice",

  // 系统
  Proxy = "proxy",
  Storage = "storage",
  Security = "security",
  ExternalTools = "external-tools",
  Experimental = "experimental",
  Developer = "developer",
  About = "about",
}

/**
 * 分组信息
 */
export interface SettingsGroupInfo {
  key: SettingsGroupKey;
  labelKey: string; // i18n key
}

/**
 * 标签页信息
 */
export interface SettingsTabInfo {
  key: SettingsTabs;
  labelKey: string; // i18n key
  group: SettingsGroupKey;
  experimental?: boolean;
}

/**
 * 分组到标签页的映射
 */
export const SETTINGS_GROUPS: Record<SettingsGroupKey, SettingsTabs[]> = {
  [SettingsGroupKey.Account]: [SettingsTabs.Profile, SettingsTabs.Stats],
  [SettingsGroupKey.General]: [
    SettingsTabs.Appearance,
    SettingsTabs.ChatAppearance,
    SettingsTabs.Hotkeys,
  ],
  [SettingsGroupKey.Agent]: [
    SettingsTabs.Providers,
    SettingsTabs.Assistant,
    SettingsTabs.Skills,
    SettingsTabs.Memory,
    SettingsTabs.ImageGen,
    SettingsTabs.Voice,
  ],
  [SettingsGroupKey.System]: [
    SettingsTabs.Proxy,
    SettingsTabs.Storage,
    SettingsTabs.Security,
    SettingsTabs.ExternalTools,
    SettingsTabs.Experimental,
    SettingsTabs.Developer,
    SettingsTabs.About,
  ],
};
