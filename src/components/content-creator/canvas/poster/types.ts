/**
 * @file 海报画布类型定义
 * @description 定义海报画布相关的核心类型，包括尺寸预设、图层、页面和画布状态
 * @module components/content-creator/canvas/poster/types
 */

import type { fabric } from "fabric";

/**
 * 海报尺寸预设类型
 */
export type PosterSizePreset =
  | "xiaohongshu-square" // 1:1, 1080×1080
  | "xiaohongshu-portrait" // 3:4, 1080×1440
  | "wechat-cover" // 2.35:1, 900×383
  | "custom";

/**
 * 尺寸配置
 */
export interface SizeConfig {
  /** 预设 ID */
  id: PosterSizePreset;
  /** 显示名称 */
  name: string;
  /** 宽度（像素） */
  width: number;
  /** 高度（像素） */
  height: number;
  /** 描述 */
  description: string;
}

/**
 * 预设尺寸列表
 */
export const SIZE_PRESETS: SizeConfig[] = [
  {
    id: "xiaohongshu-square",
    name: "小红书封面",
    width: 1080,
    height: 1080,
    description: "1:1 正方形",
  },
  {
    id: "xiaohongshu-portrait",
    name: "小红书长图",
    width: 1080,
    height: 1440,
    description: "3:4 竖版",
  },
  {
    id: "wechat-cover",
    name: "公众号头图",
    width: 900,
    height: 383,
    description: "2.35:1 横版",
  },
];

/**
 * 元素类型
 */
export type ElementType = "text" | "image" | "shape" | "group";

/**
 * 形状类型
 */
export type ShapeType = "rect" | "circle" | "triangle" | "line";

/**
 * 图层信息
 */
export interface Layer {
  /** 图层 ID */
  id: string;
  /** 图层名称 */
  name: string;
  /** 元素类型 */
  type: ElementType;
  /** 是否可见 */
  visible: boolean;
  /** 是否锁定 */
  locked: boolean;
  /** Fabric.js 对象引用（运行时） */
  fabricObjectRef?: fabric.Object;
}

/**
 * 背景类型
 */
export type BackgroundType = "solid" | "gradient" | "image";

/**
 * 渐变类型
 */
export type GradientType = "linear" | "radial";

/**
 * 背景配置
 */
export interface BackgroundConfig {
  /** 背景类型 */
  type: BackgroundType;
  /** 纯色背景颜色 */
  color?: string;
  /** 渐变配置 */
  gradient?: {
    type: GradientType;
    colors: string[];
    angle?: number; // 线性渐变角度
  };
  /** 图片背景 URL */
  imageUrl?: string;
  /** 图片填充模式 */
  imageFit?: "fill" | "fit" | "tile";
}

/**
 * 海报页面
 */
export interface PosterPage {
  /** 页面 ID */
  id: string;
  /** 页面名称 */
  name: string;
  /** 画布宽度 */
  width: number;
  /** 画布高度 */
  height: number;
  /** 背景颜色 */
  backgroundColor: string;
  /** 背景图片 URL */
  backgroundImage?: string;
  /** 背景配置 */
  background?: BackgroundConfig;
  /** 图层列表 */
  layers: Layer[];
  /** Fabric.js 序列化数据 */
  fabricJson?: string;
}

/**
 * 海报画布状态
 */
export interface PosterCanvasState {
  /** 画布类型标识 */
  type: "poster";
  /** 页面列表 */
  pages: PosterPage[];
  /** 当前页面索引 */
  currentPageIndex: number;
  /** 选中的图层 ID 列表 */
  selectedLayerIds: string[];
  /** 缩放比例（百分比） */
  zoom: number;
  /** 是否显示网格 */
  showGrid: boolean;
  /** 是否显示对齐辅助线 */
  showGuides: boolean;
}

/**
 * 海报画布 Props
 */
export interface PosterCanvasProps {
  /** 画布状态 */
  state: PosterCanvasState;
  /** 状态变更回调 */
  onStateChange: (state: PosterCanvasState) => void;
  /** 返回首页回调 */
  onBackHome?: () => void;
  /** 关闭画布回调 */
  onClose: () => void;
}

/**
 * 导出格式
 */
export type ExportFormat = "png" | "jpg";

/**
 * 导出配置
 */
export interface ExportConfig {
  /** 导出格式 */
  format: ExportFormat;
  /** 图片质量（60-100，仅 jpg） */
  quality: number;
  /** 导出倍率 */
  scale: 1 | 2 | 3;
  /** 要导出的页面索引列表 */
  pageIndices: number[];
}

/**
 * 工具栏 Props
 */
export interface PosterToolbarProps {
  /** 当前缩放比例 */
  zoom: number;
  /** 是否显示网格 */
  showGrid: boolean;
  /** 是否可以撤销 */
  canUndo: boolean;
  /** 是否可以重做 */
  canRedo: boolean;
  /** 当前画布宽度 */
  canvasWidth: number;
  /** 当前画布高度 */
  canvasHeight: number;
  /** 缩放变更回调 */
  onZoomChange: (zoom: number) => void;
  /** 网格开关回调 */
  onToggleGrid: () => void;
  /** 图层面板开关回调 */
  onToggleLayerPanel: () => void;
  /** 撤销回调 */
  onUndo: () => void;
  /** 重做回调 */
  onRedo: () => void;
  /** 导出回调 */
  onExport: () => void;
  /** 尺寸变更回调 */
  onSizeChange: (width: number, height: number) => void;
  /** 关闭回调 */
  onClose: () => void;
}

/**
 * 元素工具栏 Props
 */
export interface ElementToolbarProps {
  /** 添加文字回调 */
  onAddText: () => void;
  /** 添加图片回调 */
  onAddImage: () => void;
  /** 添加形状回调 */
  onAddShape: (type: ShapeType) => void;
  /** 设置背景回调 */
  onSetBackground: () => void;
  /** 是否有选中元素 */
  hasSelection?: boolean;
  /** 是否启用网格吸附 */
  gridSnapEnabled?: boolean;
  /** 对齐回调 */
  onAlign?: (direction: string) => void;
  /** 切换网格吸附回调 */
  onToggleGridSnap?: () => void;
}

/**
 * 图层面板 Props
 */
export interface LayerPanelProps {
  /** 图层列表 */
  layers: Layer[];
  /** 选中的图层 ID 列表 */
  selectedIds: string[];
  /** 选择图层回调 */
  onSelect: (ids: string[]) => void;
  /** 重排序回调 */
  onReorder: (fromIndex: number, toIndex: number) => void;
  /** 切换可见性回调 */
  onToggleVisibility: (id: string) => void;
  /** 切换锁定回调 */
  onToggleLock: (id: string) => void;
  /** 重命名回调 */
  onRename: (id: string, name: string) => void;
  /** 关闭面板回调 */
  onClose: () => void;
}

/**
 * 页面列表 Props
 */
export interface PageListProps {
  /** 页面列表 */
  pages: PosterPage[];
  /** 当前页面索引 */
  currentIndex: number;
  /** 页面选择回调 */
  onPageSelect: (index: number) => void;
  /** 添加页面回调 */
  onAddPage: () => void;
  /** 删除页面回调 */
  onDeletePage: (index: number) => void;
  /** 复制页面回调 */
  onDuplicatePage: (index: number) => void;
  /** 页面排序回调 */
  onReorderPages: (fromIndex: number, toIndex: number) => void;
}

/**
 * 尺寸选择器 Props
 */
export interface SizeSelectorProps {
  /** 当前宽度 */
  width: number;
  /** 当前高度 */
  height: number;
  /** 尺寸变更回调 */
  onSizeChange: (width: number, height: number) => void;
  /** 关闭回调 */
  onClose: () => void;
}

/**
 * 导出对话框 Props
 */
export interface ExportDialogProps {
  /** 是否打开 */
  open: boolean;
  /** 页面列表 */
  pages: PosterPage[];
  /** 导出回调 */
  onExport: (config: ExportConfig) => void;
  /** 关闭回调 */
  onClose: () => void;
}

/**
 * 缩放范围常量
 */
export const ZOOM_MIN = 10;
export const ZOOM_MAX = 200;
export const ZOOM_STEP = 10;

/**
 * 预设缩放值
 */
export const ZOOM_PRESETS = [25, 50, 75, 100, 125, 150, 200];

/**
 * 创建初始海报页面
 */
export function createInitialPage(
  width: number = 1080,
  height: number = 1080,
  name: string = "第 1 页",
): PosterPage {
  return {
    id: crypto.randomUUID(),
    name,
    width,
    height,
    backgroundColor: "#ffffff",
    layers: [],
  };
}

/**
 * 创建初始海报状态
 */
export function createInitialPosterState(
  width: number = 1080,
  height: number = 1080,
): PosterCanvasState {
  const initialPage = createInitialPage(width, height);

  return {
    type: "poster",
    pages: [initialPage],
    currentPageIndex: 0,
    selectedLayerIds: [],
    zoom: 100,
    showGrid: false,
    showGuides: true,
  };
}

/**
 * 限制缩放值在有效范围内
 */
export function clampZoom(zoom: number): number {
  return Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, zoom));
}
