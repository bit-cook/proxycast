/**
 * 素材相关类型定义
 *
 * 定义素材（Material）相关的 TypeScript 类型。
 *
 * @module types/material
 * @requirements 7.3
 */

// ============================================================================
// 素材类型
// ============================================================================

/**
 * 素材类型枚举
 */
export type MaterialType =
  | "document"
  | "image"
  | "audio"
  | "video"
  | "text"
  | "data"
  | "link"
  | "icon"
  | "color"
  | "layout";

/**
 * 素材类型显示名称映射
 */
export const MaterialTypeLabels: Record<MaterialType, string> = {
  document: "文档",
  image: "图片",
  audio: "语音",
  video: "视频",
  text: "文本",
  data: "数据",
  link: "链接",
  icon: "图标",
  color: "配色",
  layout: "布局",
};

/**
 * 素材
 */
export interface Material {
  id: string;
  projectId: string;
  name: string;
  type: MaterialType;
  filePath?: string;
  fileSize?: number;
  mimeType?: string;
  content?: string;
  tags: string[];
  description?: string;
  createdAt: number;
}

/**
 * 上传素材请求
 */
export interface UploadMaterialRequest {
  projectId: string;
  name: string;
  type: MaterialType;
  filePath?: string;
  content?: string;
  tags?: string[];
  description?: string;
}

/**
 * 更新素材请求
 */
export interface MaterialUpdate {
  name?: string;
  tags?: string[];
  description?: string;
}

/**
 * 素材筛选条件
 */
export interface MaterialFilter {
  type?: MaterialType;
  tags?: string[];
  searchQuery?: string;
}
