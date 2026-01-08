/**
 * 小部件相关 Hooks
 *
 * 提供小部件上下文访问和工具函数
 *
 * @module widgets/useWidgets
 */

import { useContext } from "react";
import { WidgetContext } from "./context";
import { WidgetConfig, WidgetContextValue } from "./types";

/**
 * 使用小部件上下文的 Hook
 */
export function useWidgetContext(): WidgetContextValue {
  const context = useContext(WidgetContext);
  if (!context) {
    throw new Error("useWidgetContext must be used within a WidgetProvider");
  }
  return context;
}

/**
 * 获取可见的小部件列表（按显示顺序排序）
 */
export function useVisibleWidgets(): WidgetConfig[] {
  const { widgets } = useWidgetContext();
  return widgets
    .filter((w) => !w.hidden)
    .sort((a, b) => a.displayOrder - b.displayOrder);
}
