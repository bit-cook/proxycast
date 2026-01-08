/**
 * 单个小部件按钮组件
 *
 * 渲染小部件图标和标签，支持三种显示模式
 * 包含 hover 效果和 tooltip 提示
 *
 * @module widgets/Widget
 */

import { useRef, useState, useEffect, memo } from "react";
import styled from "styled-components";
import * as LucideIcons from "lucide-react";
import { LucideIcon } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { WidgetConfig, WidgetDisplayMode } from "./types";

interface WidgetProps {
  /** 小部件配置 */
  config: WidgetConfig;
  /** 显示模式 */
  mode: WidgetDisplayMode;
  /** 点击回调 */
  onClick: () => void;
  /** 是否激活 */
  isActive?: boolean;
}

const WidgetButton = styled.button<{
  $mode: WidgetDisplayMode;
  $isActive?: boolean;
  $color?: string;
}>`
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  width: 100%;
  padding: ${({ $mode }) => ($mode === "supercompact" ? "6px 2px" : "8px 2px")};
  border: none;
  border-radius: 6px;
  background: ${({ $isActive }) =>
    $isActive ? "hsl(var(--primary) / 0.15)" : "transparent"};
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: all 0.15s ease;
  overflow: hidden;

  &:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }

  svg {
    width: ${({ $mode }) => ($mode === "supercompact" ? "16px" : "20px")};
    height: ${({ $mode }) => ($mode === "supercompact" ? "16px" : "20px")};
    color: ${({ $color, $isActive }) =>
      $isActive ? $color || "hsl(var(--primary))" : "currentColor"};
    transition: color 0.15s ease;
  }

  &:hover svg {
    color: ${({ $color }) => $color || "hsl(var(--foreground))"};
  }
`;

const WidgetLabel = styled.span`
  font-size: 10px;
  margin-top: 2px;
  width: 100%;
  padding: 0 2px;
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
`;

/**
 * 根据图标名称获取 Lucide 图标组件
 */
function getIconByName(iconName: string): LucideIcon {
  const IconComponent = (
    LucideIcons as unknown as Record<string, LucideIcon | undefined>
  )[iconName];
  return IconComponent || LucideIcons.Activity;
}

/**
 * 单个小部件按钮
 */
export const Widget = memo(function Widget({
  config,
  mode,
  onClick,
  isActive,
}: WidgetProps) {
  const [isTruncated, setIsTruncated] = useState(false);
  const labelRef = useRef<HTMLSpanElement>(null);
  const Icon = getIconByName(config.icon);

  // 检测标签是否被截断
  useEffect(() => {
    if (mode === "normal" && labelRef.current) {
      const element = labelRef.current;
      setIsTruncated(element.scrollWidth > element.clientWidth);
    }
  }, [mode, config.label]);

  // 只有在 normal 模式下标签被截断时才禁用 tooltip
  const shouldShowTooltip = mode !== "normal" || isTruncated;
  const tooltipContent = config.description || config.label;

  const button = (
    <WidgetButton
      $mode={mode}
      $isActive={isActive}
      $color={config.color}
      onClick={onClick}
    >
      <Icon />
      {mode === "normal" && config.label && (
        <WidgetLabel ref={labelRef}>{config.label}</WidgetLabel>
      )}
    </WidgetButton>
  );

  if (!shouldShowTooltip) {
    return button;
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>{button}</TooltipTrigger>
      <TooltipContent side="left">
        <span className="whitespace-nowrap">{tooltipContent}</span>
      </TooltipContent>
    </Tooltip>
  );
});
