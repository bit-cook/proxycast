/**
 * 右侧小部件栏组件
 *
 * 显示在主内容区右侧的垂直工具栏
 * 支持响应式模式切换（normal/compact/supercompact）
 * 包含小部件按钮和设置菜单
 *
 * @module widgets/WidgetsSidebar
 */

import { memo, useRef, useState, useEffect, useCallback } from "react";
import styled from "styled-components";
import { Settings } from "lucide-react";
import {
  TooltipProvider,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";
import { Widget } from "./Widget";
import { SettingsFloatingMenu } from "./SettingsFloatingMenu";
import { useVisibleWidgets, useWidgetContext } from "./useWidgets";
import { WidgetDisplayMode, WidgetType } from "./types";
import {
  WIDGETS_SIDEBAR_WIDTH,
  MODE_SWITCH_GRACE_PERIOD,
  MIN_HEIGHT_PER_WIDGET,
} from "./constants";

interface WidgetsSidebarProps {
  /** 小部件点击回调 */
  onWidgetClick: (type: WidgetType) => void;
}

const Container = styled.div`
  display: flex;
  flex-direction: column;
  width: ${WIDGETS_SIDEBAR_WIDTH}px;
  min-width: ${WIDGETS_SIDEBAR_WIDTH}px;
  height: 100%;
  padding: 8px 4px;
  background: hsl(var(--card));
  border-left: 1px solid hsl(var(--border));
  overflow: hidden;
  user-select: none;
`;

const WidgetsContainer = styled.div<{ $mode: WidgetDisplayMode }>`
  display: ${({ $mode }) => ($mode === "supercompact" ? "grid" : "flex")};
  flex-direction: column;
  grid-template-columns: ${({ $mode }) =>
    $mode === "supercompact" ? "repeat(2, 1fr)" : "none"};
  gap: 2px;
  flex: 1;
  overflow: hidden;
`;

const Spacer = styled.div`
  flex: 1;
`;

const BottomSection = styled.div<{ $mode: WidgetDisplayMode }>`
  display: ${({ $mode }) => ($mode === "supercompact" ? "grid" : "flex")};
  flex-direction: column;
  grid-template-columns: ${({ $mode }) =>
    $mode === "supercompact" ? "repeat(2, 1fr)" : "none"};
  gap: 2px;
  padding-top: 8px;
  border-top: 1px solid hsl(var(--border));
  margin-top: 8px;
`;

const SettingsButton = styled.button<{
  $mode: WidgetDisplayMode;
  $isOpen?: boolean;
}>`
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  width: 100%;
  padding: ${({ $mode }) => ($mode === "supercompact" ? "6px 2px" : "8px 2px")};
  border: none;
  border-radius: 6px;
  background: ${({ $isOpen }) =>
    $isOpen ? "hsl(var(--muted))" : "transparent"};
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }

  svg {
    width: ${({ $mode }) => ($mode === "supercompact" ? "16px" : "20px")};
    height: ${({ $mode }) => ($mode === "supercompact" ? "16px" : "20px")};
  }
`;

const SettingsLabel = styled.span`
  font-size: 10px;
  margin-top: 2px;
`;

/** 隐藏的测量容器，用于计算 normal 模式所需高度 */
const MeasurementContainer = styled.div`
  position: absolute;
  z-index: -1;
  opacity: 0;
  pointer-events: none;
  display: flex;
  flex-direction: column;
  width: ${WIDGETS_SIDEBAR_WIDTH}px;
  padding: 8px 4px;
`;

/**
 * 右侧小部件栏
 */
export const WidgetsSidebar = memo(function WidgetsSidebar({
  onWidgetClick,
}: WidgetsSidebarProps) {
  const widgets = useVisibleWidgets();
  const { activeWidget } = useWidgetContext();
  const [mode, setMode] = useState<WidgetDisplayMode>("normal");
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const measurementRef = useRef<HTMLDivElement>(null);
  const settingsButtonRef = useRef<HTMLButtonElement>(null);

  /**
   * 检查并更新显示模式
   */
  const checkModeNeeded = useCallback(() => {
    if (!containerRef.current || !measurementRef.current) return;

    const containerHeight = containerRef.current.clientHeight;
    const normalHeight = measurementRef.current.scrollHeight;

    let newMode: WidgetDisplayMode = "normal";

    // 如果 normal 模式高度超过容器高度，切换到 compact
    if (normalHeight > containerHeight - MODE_SWITCH_GRACE_PERIOD) {
      newMode = "compact";

      // 计算 supercompact 模式阈值
      const totalWidgets = widgets.length + 1; // +1 for settings
      const requiredHeight = totalWidgets * MIN_HEIGHT_PER_WIDGET;

      if (requiredHeight > containerHeight) {
        newMode = "supercompact";
      }
    }

    if (newMode !== mode) {
      setMode(newMode);
    }
  }, [mode, widgets.length]);

  // 监听容器尺寸变化
  useEffect(() => {
    const resizeObserver = new ResizeObserver(() => {
      checkModeNeeded();
    });

    if (containerRef.current) {
      resizeObserver.observe(containerRef.current);
    }

    return () => {
      resizeObserver.disconnect();
    };
  }, [checkModeNeeded]);

  // 小部件数量变化时重新检查
  useEffect(() => {
    checkModeNeeded();
  }, [widgets, checkModeNeeded]);

  const handleSettingsMenuItemClick = useCallback(
    (type: WidgetType) => {
      onWidgetClick(type);
    },
    [onWidgetClick],
  );

  return (
    <TooltipProvider>
      <Container ref={containerRef}>
        <WidgetsContainer $mode={mode}>
          {widgets.map((widget) => (
            <Widget
              key={widget.id}
              config={widget}
              mode={mode}
              onClick={() => onWidgetClick(widget.type)}
              isActive={activeWidget === widget.type}
            />
          ))}
        </WidgetsContainer>

        {mode !== "supercompact" && <Spacer />}

        <BottomSection $mode={mode}>
          <Tooltip>
            <TooltipTrigger asChild>
              <SettingsButton
                ref={settingsButtonRef}
                $mode={mode}
                $isOpen={isSettingsOpen}
                onClick={() => setIsSettingsOpen(!isSettingsOpen)}
              >
                <Settings />
                {mode === "normal" && <SettingsLabel>设置</SettingsLabel>}
              </SettingsButton>
            </TooltipTrigger>
            {!isSettingsOpen && (
              <TooltipContent side="left">
                <span className="whitespace-nowrap">设置与帮助</span>
              </TooltipContent>
            )}
          </Tooltip>
        </BottomSection>

        <SettingsFloatingMenu
          isOpen={isSettingsOpen}
          onClose={() => setIsSettingsOpen(false)}
          referenceElement={settingsButtonRef.current}
          onMenuItemClick={handleSettingsMenuItemClick}
        />

        {/* 隐藏的测量容器 */}
        <MeasurementContainer ref={measurementRef}>
          {widgets.map((widget) => (
            <Widget
              key={`measure-${widget.id}`}
              config={widget}
              mode="normal"
              onClick={() => {}}
            />
          ))}
          <Spacer />
          <SettingsButton $mode="normal">
            <Settings />
            <SettingsLabel>设置</SettingsLabel>
          </SettingsButton>
        </MeasurementContainer>
      </Container>
    </TooltipProvider>
  );
});
