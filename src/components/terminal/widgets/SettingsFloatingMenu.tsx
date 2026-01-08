/**
 * 设置浮动菜单组件
 *
 * 点击设置按钮后显示的浮动菜单
 * 包含 Settings、Tips、Secrets、Help 菜单项
 * 使用 @floating-ui/react 进行定位
 *
 * @module widgets/SettingsFloatingMenu
 */

import { memo } from "react";
import styled from "styled-components";
import {
  useFloating,
  useDismiss,
  useInteractions,
  offset,
  shift,
  autoUpdate,
  FloatingPortal,
} from "@floating-ui/react";
import * as LucideIcons from "lucide-react";
import { LucideIcon } from "lucide-react";
import { WidgetType } from "./types";
import { SETTINGS_MENU_ITEMS } from "./constants";

interface SettingsFloatingMenuProps {
  /** 是否打开 */
  isOpen: boolean;
  /** 关闭回调 */
  onClose: () => void;
  /** 参考元素（触发按钮） */
  referenceElement: HTMLElement | null;
  /** 菜单项点击回调 */
  onMenuItemClick: (type: WidgetType) => void;
}

const MenuContainer = styled.div`
  background: hsl(var(--popover));
  border: 1px solid hsl(var(--border));
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  padding: 6px;
  z-index: 50;
  min-width: 140px;
`;

const MenuItem = styled.button`
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: all 0.15s ease;
  font-size: 13px;

  &:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }

  svg {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }
`;

const MenuLabel = styled.span`
  white-space: nowrap;
`;

/**
 * 根据图标名称获取 Lucide 图标组件
 */
function getIconByName(iconName: string): LucideIcon {
  const IconComponent = (
    LucideIcons as unknown as Record<string, LucideIcon | undefined>
  )[iconName];
  return IconComponent || LucideIcons.Settings;
}

/**
 * 设置浮动菜单
 */
export const SettingsFloatingMenu = memo(function SettingsFloatingMenu({
  isOpen,
  onClose,
  referenceElement,
  onMenuItemClick,
}: SettingsFloatingMenuProps) {
  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: (open) => {
      if (!open) onClose();
    },
    placement: "left-start",
    middleware: [offset(-2), shift({ padding: 12 })],
    whileElementsMounted: autoUpdate,
    elements: {
      reference: referenceElement,
    },
  });

  const dismiss = useDismiss(context);
  const { getFloatingProps } = useInteractions([dismiss]);

  if (!isOpen || !referenceElement) {
    return null;
  }

  return (
    <FloatingPortal>
      <MenuContainer
        ref={refs.setFloating}
        style={floatingStyles}
        {...getFloatingProps()}
      >
        {SETTINGS_MENU_ITEMS.map((item) => {
          const Icon = getIconByName(item.icon);
          return (
            <MenuItem
              key={item.id}
              onClick={() => {
                onMenuItemClick(item.type);
                onClose();
              }}
            >
              <Icon />
              <MenuLabel>{item.label}</MenuLabel>
            </MenuItem>
          );
        })}
      </MenuContainer>
    </FloatingPortal>
  );
});
