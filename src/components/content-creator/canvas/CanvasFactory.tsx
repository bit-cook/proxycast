/**
 * @file 画布工厂组件
 * @description 根据主题类型动态渲染对应的画布组件
 * @module components/content-creator/canvas/CanvasFactory
 */

import React, { memo, useMemo } from "react";
import type { ThemeType } from "../types";
import { DocumentCanvas } from "./document";
import type { DocumentCanvasState } from "./document/types";
import { PosterCanvas } from "./poster";
import type { PosterCanvasState } from "./poster/types";
import { MusicCanvas } from "./music";
import type { MusicCanvasState } from "./music/types";
import { ScriptCanvas } from "./script";
import type { ScriptCanvasState } from "./script/types";
import { NovelCanvas } from "./novel";
import type { NovelCanvasState } from "./novel/types";
import { VideoCanvas } from "./video";
import type { VideoCanvasState } from "./video/types";
import { getCanvasTypeForTheme, type CanvasStateUnion } from "./canvasUtils";

/**
 * 画布工厂 Props
 */
interface CanvasFactoryProps {
  /** 当前主题 */
  theme: ThemeType;
  /** 画布状态 */
  state: CanvasStateUnion;
  /** 状态变更回调 */
  onStateChange: (state: CanvasStateUnion) => void;
  /** 关闭回调 */
  onClose: () => void;
  /** 返回首页回调 */
  onBackHome?: () => void;
  /** 是否正在流式输出（仅文档画布使用） */
  isStreaming?: boolean;
  /** 小说画布控制配置（可选） */
  novelControls?: {
    /** 是否由外部导航栏接管控制按钮 */
    useExternalToolbar: boolean;
    /** 章节栏是否折叠 */
    chapterListCollapsed: boolean;
    /** 章节栏折叠状态变更 */
    onChapterListCollapsedChange: (collapsed: boolean) => void;
  } | null;
  /** 画布选中文本变更 */
  onSelectionTextChange?: (text: string) => void;
}

/**
 * 画布工厂组件
 *
 * 根据画布状态类型动态渲染对应的画布组件
 * 优先使用 state.type 来决定渲染哪个画布，以支持 general 等主题
 */
export const CanvasFactory: React.FC<CanvasFactoryProps> = memo(
  ({
    theme,
    state,
    onStateChange,
    onClose,
    onBackHome,
    isStreaming,
    novelControls,
    onSelectionTextChange,
  }) => {
    const resolvedBackHome = onBackHome ?? onClose;

    // 优先根据 state.type 渲染，这样 general 主题也能显示文档画布
    // 只有当 state.type 与 theme 对应的 canvasType 不匹配时才检查 theme
    const canvasType = useMemo(() => {
      // 如果 state 有明确的类型，直接使用
      if (state.type) {
        return state.type;
      }
      // 否则根据 theme 获取
      return getCanvasTypeForTheme(theme);
    }, [theme, state.type]);

    // 根据画布类型渲染对应组件
    if (canvasType === "document" && state.type === "document") {
      return (
        <DocumentCanvas
          state={state}
          onStateChange={onStateChange as (s: DocumentCanvasState) => void}
          onBackHome={resolvedBackHome}
          onClose={onClose}
          isStreaming={isStreaming}
          onSelectionTextChange={onSelectionTextChange}
        />
      );
    }

    if (canvasType === "poster" && state.type === "poster") {
      return (
        <PosterCanvas
          state={state}
          onStateChange={onStateChange as (s: PosterCanvasState) => void}
          onBackHome={resolvedBackHome}
          onClose={onClose}
        />
      );
    }

    if (canvasType === "music" && state.type === "music") {
      return (
        <MusicCanvas
          state={state}
          onStateChange={onStateChange as (s: MusicCanvasState) => void}
          onBackHome={resolvedBackHome}
          onClose={onClose}
          isStreaming={isStreaming}
        />
      );
    }

    if (canvasType === "script" && state.type === "script") {
      return (
        <ScriptCanvas
          state={state}
          onStateChange={onStateChange as (s: ScriptCanvasState) => void}
          onBackHome={resolvedBackHome}
          onClose={onClose}
        />
      );
    }

    if (canvasType === "novel" && state.type === "novel") {
      return (
        <NovelCanvas
          state={state}
          onStateChange={onStateChange as (s: NovelCanvasState) => void}
          onBackHome={resolvedBackHome}
          onClose={onClose}
          useExternalToolbar={novelControls?.useExternalToolbar}
          chapterListCollapsed={novelControls?.chapterListCollapsed}
          onChapterListCollapsedChange={
            novelControls?.onChapterListCollapsedChange
          }
          onSelectionTextChange={onSelectionTextChange}
        />
      );
    }

    if (canvasType === "video" && state.type === "video") {
      return (
        <VideoCanvas
          state={state}
          onStateChange={onStateChange as (s: VideoCanvasState) => void}
          onBackHome={resolvedBackHome}
          onClose={onClose}
        />
      );
    }

    // 不支持的主题或状态类型不匹配
    return null;
  },
);

CanvasFactory.displayName = "CanvasFactory";
