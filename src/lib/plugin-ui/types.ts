/**
 * @file ProxyCast Plugin UI 类型定义
 * @description 基于 A2UI 设计理念的声明式插件 UI 系统类型
 * @module lib/plugin-ui/types
 */

import type React from "react";

// ============================================================================
// 基础类型
// ============================================================================

/** 组件 ID */
export type ComponentId = string;

/** Surface ID */
export type SurfaceId = string;

/** 插件 ID */
export type PluginId = string;

/** 数据路径 (JSONPath 格式) */
export type DataPath = string;

// ============================================================================
// 数据绑定
// ============================================================================

/** 绑定值 - 支持字面值或数据路径绑定 */
export type BoundValue<T> =
  | { literalString: string }
  | { literalNumber: number }
  | { literalBoolean: boolean }
  | { literalArray: T[] }
  | { path: DataPath }
  | { literalString: string; path: DataPath }
  | { literalNumber: number; path: DataPath }
  | { literalBoolean: boolean; path: DataPath };

/** 字符串绑定值 */
export type BoundString = BoundValue<string>;

/** 数字绑定值 */
export type BoundNumber = BoundValue<number>;

/** 布尔绑定值 */
export type BoundBoolean = BoundValue<boolean>;

// ============================================================================
// 子组件定义
// ============================================================================

/** 子组件列表定义 */
export interface ChildrenDef {
  /** 显式列表 - 固定的子组件 ID 列表 */
  explicitList?: ComponentId[];
  /** 模板 - 从数据列表动态生成子组件 */
  template?: {
    componentId: ComponentId;
    dataBinding: DataPath;
  };
}

// ============================================================================
// 操作定义
// ============================================================================

/** 操作上下文项 */
export interface ActionContextItem {
  key: string;
  value: BoundValue<unknown>;
}

/** 操作定义 */
export interface Action {
  name: string;
  context?: ActionContextItem[];
}

// ============================================================================
// 组件类型
// ============================================================================

/** 文本变体 */
export type TextVariant = "h1" | "h2" | "h3" | "h4" | "h5" | "body" | "caption";

/** 按钮变体 */
export type ButtonVariant =
  | "default"
  | "primary"
  | "secondary"
  | "destructive"
  | "outline"
  | "ghost";

/** Badge 变体 */
export type BadgeVariant = "default" | "success" | "warning" | "error" | "info";

/** Alert 类型 */
export type AlertType = "info" | "success" | "warning" | "error";

/** 对齐方式 */
export type Alignment = "start" | "center" | "end" | "stretch";

/** 分布方式 */
export type Distribution =
  | "start"
  | "center"
  | "end"
  | "spaceBetween"
  | "spaceAround"
  | "spaceEvenly";

/** 方向 */
export type Direction = "horizontal" | "vertical";

/** 图标名称 */
export type IconName =
  | "add"
  | "check"
  | "close"
  | "delete"
  | "edit"
  | "refresh"
  | "search"
  | "settings"
  | "info"
  | "warning"
  | "error"
  | "chevronDown"
  | "chevronUp"
  | "chevronLeft"
  | "chevronRight"
  | "power"
  | "powerOff"
  | "folder"
  | "file"
  | "copy"
  | "play"
  | "pause"
  | "stop"
  | "download"
  | "upload";

// ============================================================================
// 标准组件定义
// ============================================================================

/** Row 组件 */
export interface RowComponent {
  Row: {
    children: ChildrenDef;
    distribution?: Distribution;
    alignment?: Alignment;
    gap?: number;
  };
}

/** Column 组件 */
export interface ColumnComponent {
  Column: {
    children: ChildrenDef;
    distribution?: Distribution;
    alignment?: Alignment;
    gap?: number;
  };
}

/** Card 组件 */
export interface CardComponent {
  Card: {
    child: ComponentId;
    title?: BoundString;
    description?: BoundString;
  };
}

/** Text 组件 */
export interface TextComponent {
  Text: {
    text: BoundString;
    variant?: TextVariant;
  };
}

/** Icon 组件 */
export interface IconComponent {
  Icon: {
    name: BoundValue<IconName>;
    size?: number;
    color?: string;
  };
}

/** Button 组件 */
export interface ButtonComponent {
  Button: {
    child: ComponentId;
    action: Action;
    variant?: ButtonVariant;
    disabled?: BoundBoolean;
  };
}

/** Badge 组件 */
export interface BadgeComponent {
  Badge: {
    text: BoundString;
    variant?: BoundValue<BadgeVariant>;
  };
}

/** Progress 组件 */
export interface ProgressComponent {
  Progress: {
    value: BoundNumber;
    max?: number;
  };
}

/** TextField 组件 */
export interface TextFieldComponent {
  TextField: {
    label: BoundString;
    value: BoundString;
    placeholder?: BoundString;
    type?: "text" | "password" | "number" | "email";
    disabled?: BoundBoolean;
  };
}

/** Switch 组件 */
export interface SwitchComponent {
  Switch: {
    label: BoundString;
    checked: BoundBoolean;
    disabled?: BoundBoolean;
  };
}

/** Select 组件 */
export interface SelectComponent {
  Select: {
    label?: BoundString;
    value: BoundString;
    options: Array<{
      label: BoundString;
      value: string;
    }>;
    disabled?: BoundBoolean;
  };
}

/** List 组件 */
export interface ListComponent {
  List: {
    children: ChildrenDef;
    direction?: Direction;
    alignment?: Alignment;
    gap?: number;
  };
}

/** Tabs 组件 */
export interface TabsComponent {
  Tabs: {
    items: Array<{
      id: string;
      title: BoundString;
      child: ComponentId;
    }>;
    defaultTab?: string;
  };
}

/** Alert 组件 */
export interface AlertComponent {
  Alert: {
    message: BoundString;
    type: AlertType;
    title?: BoundString;
  };
}

/** Spinner 组件 */
export interface SpinnerComponent {
  Spinner: {
    size?: number;
  };
}

/** Empty 组件 */
export interface EmptyComponent {
  Empty: {
    icon?: IconName;
    title?: BoundString;
    description?: BoundString;
  };
}

/** Divider 组件 */
export interface DividerComponent {
  Divider: {
    axis?: "horizontal" | "vertical";
  };
}

/** KeyValue 组件 */
export interface KeyValueComponent {
  KeyValue: {
    items: Array<{
      key: BoundString;
      value: BoundString;
    }>;
  };
}

/** 所有组件类型联合 */
export type ComponentType =
  | RowComponent
  | ColumnComponent
  | CardComponent
  | TextComponent
  | IconComponent
  | ButtonComponent
  | BadgeComponent
  | ProgressComponent
  | TextFieldComponent
  | SwitchComponent
  | SelectComponent
  | ListComponent
  | TabsComponent
  | AlertComponent
  | SpinnerComponent
  | EmptyComponent
  | DividerComponent
  | KeyValueComponent;

/** 组件类型名称 */
export type ComponentTypeName = keyof (RowComponent &
  ColumnComponent &
  CardComponent &
  TextComponent &
  IconComponent &
  ButtonComponent &
  BadgeComponent &
  ProgressComponent &
  TextFieldComponent &
  SwitchComponent &
  SelectComponent &
  ListComponent &
  TabsComponent &
  AlertComponent &
  SpinnerComponent &
  EmptyComponent &
  DividerComponent &
  KeyValueComponent);

// ============================================================================
// 组件定义
// ============================================================================

/** 组件定义 */
export interface ComponentDef {
  id: ComponentId;
  component: ComponentType;
  weight?: number; // flex-grow 权重
}

// ============================================================================
// 消息类型 (Server → Client)
// ============================================================================

/** Surface 更新消息 */
export interface SurfaceUpdate {
  surfaceId: SurfaceId;
  components: ComponentDef[];
}

/** 数据条目 */
export interface DataEntry {
  key: string;
  valueString?: string;
  valueNumber?: number;
  valueBoolean?: boolean;
  valueArray?: DataEntry[];
  valueMap?: DataEntry[];
}

/** 数据模型更新消息 */
export interface DataModelUpdate {
  surfaceId: SurfaceId;
  path?: DataPath;
  contents: DataEntry[];
}

/** Surface 样式 */
export interface SurfaceStyles {
  primaryColor?: string;
  font?: string;
  borderRadius?: number;
}

/** 开始渲染消息 */
export interface BeginRendering {
  surfaceId: SurfaceId;
  root: ComponentId;
  catalogId?: string;
  styles?: SurfaceStyles;
}

/** 删除 Surface 消息 */
export interface DeleteSurface {
  surfaceId: SurfaceId;
}

/** 服务端消息 */
export type ServerMessage =
  | { surfaceUpdate: SurfaceUpdate }
  | { dataModelUpdate: DataModelUpdate }
  | { beginRendering: BeginRendering }
  | { deleteSurface: DeleteSurface };

/** 插件任务状态 */
export type PluginTaskState =
  | "queued"
  | "running"
  | "retrying"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "timed_out";

/** 插件任务错误 */
export interface PluginTaskError {
  code?: string;
  message: string;
  retryable: boolean;
}

/** 插件任务事件 */
export interface PluginTaskEventPayload {
  pluginId: PluginId;
  taskId: string;
  operation: string;
  state: PluginTaskState;
  attempt: number;
  timestamp: string;
  error?: PluginTaskError;
}

/** 插件任务记录 */
export interface PluginTaskRecord {
  taskId: string;
  pluginId: PluginId;
  operation: string;
  state: PluginTaskState;
  attempt: number;
  maxRetries: number;
  startedAt: string;
  endedAt?: string;
  durationMs?: number;
  error?: PluginTaskError;
}

// ============================================================================
// 消息类型 (Client → Server)
// ============================================================================

/** 用户操作消息 */
export interface UserAction {
  name: string;
  surfaceId: SurfaceId;
  sourceComponentId: ComponentId;
  context: Record<string, unknown>;
  timestamp: string;
}

/** 客户端错误消息 */
export interface ClientError {
  surfaceId: SurfaceId;
  message: string;
  componentId?: ComponentId;
}

/** 客户端消息 */
export type ClientMessage = { userAction: UserAction } | { error: ClientError };

// ============================================================================
// Surface 状态
// ============================================================================

/** Surface 状态 */
export interface SurfaceState {
  surfaceId: SurfaceId;
  pluginId: PluginId;
  rootId: ComponentId | null;
  components: Map<ComponentId, ComponentDef>;
  dataModel: Record<string, unknown>;
  styles: SurfaceStyles;
  isReady: boolean;
}

// ============================================================================
// 组件注册表
// ============================================================================

/** 组件渲染器 Props */
export interface ComponentRendererProps {
  componentDef: ComponentDef;
  surface: SurfaceState;
  onAction: (action: UserAction) => void;
  resolveValue: <T>(bound: BoundValue<T>, itemData?: unknown) => T | undefined;
  renderChild: (childId: ComponentId, itemData?: unknown) => React.ReactNode;
  renderChildren: (
    children: ChildrenDef,
    itemData?: unknown,
  ) => React.ReactNode[];
}

/** 组件渲染器类型 */
export type ComponentRenderer = React.FC<ComponentRendererProps>;

/** 组件注册表条目 */
export interface ComponentRegistryEntry {
  renderer: ComponentRenderer;
  schema?: Record<string, unknown>;
}
