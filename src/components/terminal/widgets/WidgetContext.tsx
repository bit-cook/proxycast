/**
 * 小部件上下文管理
 *
 * 提供小部件配置的全局状态管理
 * 支持配置持久化到 localStorage
 *
 * @module widgets/WidgetContext
 */

import { useState, useEffect, useCallback, ReactNode } from "react";
import { WidgetConfig, WidgetType, WidgetContextValue } from "./types";
import { DEFAULT_WIDGETS, STORAGE_KEYS } from "./constants";
import { WidgetContext } from "./context";

export { WidgetContext };

interface WidgetProviderProps {
  children: ReactNode;
}

/**
 * 从 localStorage 加载小部件配置
 */
function loadWidgetConfig(): WidgetConfig[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEYS.WIDGET_CONFIG);
    if (stored) {
      const parsed = JSON.parse(stored);
      // 合并默认配置和存储的配置
      return DEFAULT_WIDGETS.map((defaultWidget) => {
        const storedWidget = parsed.find(
          (w: WidgetConfig) => w.id === defaultWidget.id,
        );
        return storedWidget
          ? { ...defaultWidget, ...storedWidget }
          : defaultWidget;
      });
    }
  } catch (e) {
    console.error("加载小部件配置失败:", e);
  }
  return DEFAULT_WIDGETS;
}

/**
 * 保存小部件配置到 localStorage
 */
function saveWidgetConfig(widgets: WidgetConfig[]): void {
  try {
    localStorage.setItem(STORAGE_KEYS.WIDGET_CONFIG, JSON.stringify(widgets));
  } catch (e) {
    console.error("保存小部件配置失败:", e);
  }
}

/**
 * 小部件上下文 Provider
 */
export function WidgetProvider({ children }: WidgetProviderProps) {
  const [widgets, setWidgets] = useState<WidgetConfig[]>(loadWidgetConfig);
  const [activeWidget, setActiveWidget] = useState<WidgetType | null>(null);

  // 配置变化时保存到 localStorage
  useEffect(() => {
    saveWidgetConfig(widgets);
  }, [widgets]);

  const updateWidget = useCallback(
    (id: string, config: Partial<WidgetConfig>) => {
      setWidgets((prev) =>
        prev.map((widget) =>
          widget.id === id ? { ...widget, ...config } : widget,
        ),
      );
    },
    [],
  );

  const toggleWidgetVisibility = useCallback((id: string) => {
    setWidgets((prev) =>
      prev.map((widget) =>
        widget.id === id ? { ...widget, hidden: !widget.hidden } : widget,
      ),
    );
  }, []);

  const value: WidgetContextValue = {
    widgets,
    updateWidget,
    toggleWidgetVisibility,
    activeWidget,
    setActiveWidget,
  };

  return (
    <WidgetContext.Provider value={value}>{children}</WidgetContext.Provider>
  );
}
