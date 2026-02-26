import type { Skill } from "@/lib/api/skills";

/**
 * @file types.ts
 * @description 截图对话模块类型定义
 * @module components/smart-input/types
 */

// ============================================================
// 配置类型
// ============================================================

/**
 * 截图对话功能配置
 * 需求: 1.1 - 实验室功能应提供 screenshot_chat.enabled 布尔开关
 */
export interface SmartInputConfig {
  /** 是否启用截图对话功能 */
  enabled: boolean;
  /** 触发截图的全局快捷键 */
  shortcut: string;
}

// ============================================================
// 消息类型
// ============================================================

/**
 * 消息图片
 * 需求: 5.1 - 截图对话模块应将图片编码为 base64
 */
export interface MessageImage {
  /** Base64 编码的图片数据 */
  data: string;
  /** 媒体类型，如 "image/png" */
  mediaType: string;
}

/**
 * 聊天消息
 * 需求: 4.5 - 悬浮窗口应在可滚动区域显示 AI 回复
 */
export interface ChatMessage {
  /** 消息唯一标识 */
  id: string;
  /** 消息角色：用户或助手 */
  role: "user" | "assistant";
  /** 消息文本内容 */
  content: string;
  /** 附带的图片（用户消息可能包含截图） */
  image?: MessageImage;
  /** 消息时间戳 */
  timestamp: number;
  /** 是否正在思考中（助手消息） */
  isThinking?: boolean;
  /** 思考中的提示文本 */
  thinkingContent?: string;
}

// ============================================================
// Hook 状态类型
// ============================================================

/**
 * 截图对话 Hook 状态
 */
export interface SmartInputState {
  /** 消息列表 */
  messages: ChatMessage[];
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 当前截图路径 */
  imagePath: string | null;
  /** 当前截图的 Base64 编码 */
  imageBase64: string | null;
}

/**
 * 截图对话 Hook 返回值
 */
export interface UseSmartInputReturn extends SmartInputState {
  /** 发送消息到 AI */
  sendMessage: (message: string) => Promise<void>;
  /** 设置截图路径 */
  setImagePath: (path: string) => void;
  /** 清空消息历史 */
  clearMessages: () => void;
  /** 清除错误 */
  clearError: () => void;
  /** 重试上一条消息 */
  retry: () => Promise<void>;
}

// ============================================================
// 组件 Props 类型
// ============================================================

/**
 * 截图预览组件属性
 * 需求: 4.2 - 悬浮窗口应显示截图预览
 */
export interface SmartInputPreviewProps {
  /** 图片路径或 Base64 编码 */
  src: string;
  /** 图片 alt 文本 */
  alt?: string;
  /** 自定义类名 */
  className?: string;
  /** 最大高度 */
  maxHeight?: number;
}

/**
 * 聊天输入框组件属性
 * 需求: 4.3, 4.4 - 悬浮窗口应提供文本输入框，支持 Enter 发送
 */
export interface ChatInputProps {
  /** 输入框值 */
  value: string;
  /** 值变化回调 */
  onChange: (value: string) => void;
  /** 发送消息回调（可接受 textOverride） */
  onSend: (textOverride?: string) => void;
  /** 是否禁用 */
  disabled?: boolean;
  /** 是否正在加载 */
  isLoading?: boolean;
  /** 占位符文本 */
  placeholder?: string;
  /** 技能列表 */
  skills?: Skill[];
}

/**
 * 消息列表组件属性
 * 需求: 4.5, 5.4 - 显示 AI 回复，支持 Markdown 渲染
 */
export interface ChatMessagesProps {
  /** 消息列表 */
  messages: ChatMessage[];
  /** 自定义类名 */
  className?: string;
}

/**
 * 悬浮窗主组件属性
 * 需求: 4.1, 4.6, 4.7 - 无边框置顶窗口，支持 ESC 关闭和拖动
 */
export interface SmartInputWindowProps {
  /** 截图路径 */
  imagePath: string;
  /** 关闭窗口回调 */
  onClose?: () => void;
}
