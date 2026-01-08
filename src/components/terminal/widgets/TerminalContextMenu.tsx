/**
 * 终端右键菜单组件
 *
 * 借鉴 Waveterm 的终端右键菜单功能
 * 包括复制、粘贴、清屏、字体大小、主题等功能
 *
 * @module widgets/TerminalContextMenu
 */

import React, {
  memo,
  useCallback,
  useMemo,
  useEffect,
  useRef,
  useState,
} from "react";
import styled from "styled-components";
import {
  Copy,
  Clipboard,
  Trash2,
  Type,
  Palette,
  ChevronRight,
  Maximize2,
  Minimize2,
  RotateCcw,
  SplitSquareHorizontal,
  SplitSquareVertical,
  ExternalLink,
  Globe,
} from "lucide-react";
import { type ThemeName } from "@/lib/terminal/themes";

// ============================================================================
// 类型定义
// ============================================================================

/** 菜单位置 */
export interface ContextMenuPosition {
  x: number;
  y: number;
}

/** 组件属性 */
export interface TerminalContextMenuProps {
  /** 菜单位置 */
  position: ContextMenuPosition;
  /** 关闭回调 */
  onClose: () => void;
  /** 是否有选中文本 */
  hasSelection: boolean;
  /** 获取选中文本 */
  getSelection: () => string | null;
  /** 复制回调 */
  onCopy: () => void;
  /** 粘贴回调 */
  onPaste: () => void;
  /** 清屏回调 */
  onClear: () => void;
  /** 字体大小变化回调 */
  onFontSizeChange: (size: number) => void;
  /** 当前字体大小 */
  currentFontSize: number;
  /** 主题变化回调 */
  onThemeChange: (theme: ThemeName) => void;
  /** 当前主题 */
  currentTheme: ThemeName;
  /** 水平分割回调 */
  onSplitHorizontal?: () => void;
  /** 垂直分割回调 */
  onSplitVertical?: () => void;
  /** 放大/缩小回调 */
  onToggleMagnify?: () => void;
  /** 是否已放大 */
  isMagnified?: boolean;
  /** 重启终端回调 */
  onRestart?: () => void;
}

/** 菜单项 */
interface MenuItem {
  id: string;
  label: string;
  icon?: React.ReactNode;
  shortcut?: string;
  disabled?: boolean;
  danger?: boolean;
  checked?: boolean;
  onClick?: () => void;
  subItems?: MenuItem[];
}

/** 菜单分隔线 */
interface MenuDivider {
  id: string;
  type: "divider";
}

type MenuItemOrDivider = MenuItem | MenuDivider;

// ============================================================================
// 样式组件
// ============================================================================

const MenuContainer = styled.div`
  position: fixed;
  z-index: 9999;
  min-width: 200px;
  padding: 4px 0;
  background: rgb(30, 30, 30);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
`;

const MenuItemButton = styled.button<{
  $danger?: boolean;
  $disabled?: boolean;
}>`
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 12px;
  border: none;
  background: transparent;
  color: ${({ $danger, $disabled }) =>
    $disabled ? "#505050" : $danger ? "#e54d2e" : "#e0e0e0"};
  font-size: 13px;
  text-align: left;
  cursor: ${({ $disabled }) => ($disabled ? "not-allowed" : "pointer")};
  transition: background 0.1s ease;

  &:hover:not(:disabled) {
    background: ${({ $danger }) =>
      $danger ? "rgba(229, 77, 46, 0.15)" : "rgba(255, 255, 255, 0.08)"};
  }

  svg {
    width: 16px;
    height: 16px;
    opacity: 0.8;
  }
`;

const MenuItemLabel = styled.span`
  flex: 1;
`;

const MenuItemShortcut = styled.span`
  font-size: 11px;
  color: #606060;
`;

const MenuDividerLine = styled.div`
  height: 1px;
  margin: 4px 8px;
  background: rgba(255, 255, 255, 0.1);
`;

const SubMenuContainer = styled.div`
  position: absolute;
  min-width: 160px;
  padding: 4px 0;
  background: rgb(30, 30, 30);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
`;

const CheckMark = styled.span`
  width: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #58a6ff;
`;

// ============================================================================
// 可用主题列表
// ============================================================================

const AVAILABLE_THEMES: Array<{ id: ThemeName; name: string }> = [
  { id: "tokyo-night", name: "Tokyo Night" },
  { id: "dracula", name: "Dracula" },
  { id: "monokai", name: "Monokai" },
  { id: "nord", name: "Nord" },
  { id: "solarized-dark", name: "Solarized Dark" },
  { id: "one-dark", name: "One Dark" },
  { id: "github-dark", name: "GitHub Dark" },
  { id: "gruvbox-dark", name: "Gruvbox Dark" },
];

// ============================================================================
// 可用字体大小
// ============================================================================

const FONT_SIZES = [10, 11, 12, 13, 14, 15, 16, 18, 20];

// ============================================================================
// 主组件
// ============================================================================

/**
 * 终端右键菜单
 */
export const TerminalContextMenu = memo(function TerminalContextMenu({
  position,
  onClose,
  hasSelection,
  getSelection,
  onCopy,
  onPaste,
  onClear,
  onFontSizeChange,
  currentFontSize,
  onThemeChange,
  currentTheme,
  onSplitHorizontal,
  onSplitVertical,
  onToggleMagnify,
  isMagnified,
  onRestart,
}: TerminalContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [activeSubMenu, setActiveSubMenu] = useState<string | null>(null);
  const [subMenuPosition, setSubMenuPosition] = useState<{
    top: number;
    left: number;
  } | null>(null);

  // 点击外部关闭菜单
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose]);

  // 调整菜单位置，确保不超出视口
  useEffect(() => {
    if (menuRef.current) {
      const menu = menuRef.current;
      const rect = menu.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      let x = position.x;
      let y = position.y;

      if (x + rect.width > viewportWidth) {
        x = viewportWidth - rect.width - 8;
      }

      if (y + rect.height > viewportHeight) {
        y = viewportHeight - rect.height - 8;
      }

      menu.style.left = `${x}px`;
      menu.style.top = `${y}px`;
    }
  }, [position]);

  // 处理子菜单悬停
  const handleSubMenuHover = useCallback(
    (itemId: string, event: React.MouseEvent) => {
      const target = event.currentTarget as HTMLElement;
      const rect = target.getBoundingClientRect();

      setActiveSubMenu(itemId);
      setSubMenuPosition({
        top: rect.top,
        left: rect.right + 2,
      });
    },
    [],
  );

  // 处理复制
  const handleCopy = useCallback(() => {
    onCopy();
    onClose();
  }, [onCopy, onClose]);

  // 处理粘贴
  const handlePaste = useCallback(() => {
    onPaste();
    onClose();
  }, [onPaste, onClose]);

  // 处理清屏
  const handleClear = useCallback(() => {
    onClear();
    onClose();
  }, [onClear, onClose]);

  // 处理在浏览器中打开 URL
  const handleOpenUrl = useCallback(async () => {
    const selection = getSelection();
    if (selection) {
      try {
        const trimmed = selection.trim();
        const url = new URL(trimmed);
        if (url.protocol.startsWith("http")) {
          window.open(url.toString(), "_blank");
        }
      } catch {
        // 不是有效 URL
      }
    }
    onClose();
  }, [getSelection, onClose]);

  // 检查选中文本是否是 URL
  const selectionUrl = useMemo(() => {
    if (!hasSelection) return null;
    const selection = getSelection();
    if (!selection) return null;
    try {
      const trimmed = selection.trim();
      const url = new URL(trimmed);
      if (url.protocol.startsWith("http")) {
        return url;
      }
    } catch {
      // 不是有效 URL
    }
    return null;
  }, [hasSelection, getSelection]);

  // 构建菜单项
  const menuItems: MenuItemOrDivider[] = useMemo(() => {
    const items: MenuItemOrDivider[] = [];

    // 复制（仅当有选中文本时启用）
    items.push({
      id: "copy",
      label: "复制",
      icon: <Copy />,
      shortcut: "⌘C",
      disabled: !hasSelection,
      onClick: handleCopy,
    });

    // 粘贴
    items.push({
      id: "paste",
      label: "粘贴",
      icon: <Clipboard />,
      shortcut: "⌘V",
      onClick: handlePaste,
    });

    items.push({ id: "divider-1", type: "divider" });

    // 如果选中的是 URL，显示打开选项
    if (selectionUrl) {
      items.push({
        id: "open-url",
        label: `打开 URL (${selectionUrl.hostname})`,
        icon: <Globe />,
        onClick: handleOpenUrl,
      });

      items.push({
        id: "open-url-external",
        label: "在外部浏览器中打开",
        icon: <ExternalLink />,
        onClick: handleOpenUrl,
      });

      items.push({ id: "divider-url", type: "divider" });
    }

    // 分割面板
    if (onSplitHorizontal) {
      items.push({
        id: "split-horizontal",
        label: "水平分割",
        icon: <SplitSquareHorizontal />,
        onClick: () => {
          onSplitHorizontal();
          onClose();
        },
      });
    }

    if (onSplitVertical) {
      items.push({
        id: "split-vertical",
        label: "垂直分割",
        icon: <SplitSquareVertical />,
        onClick: () => {
          onSplitVertical();
          onClose();
        },
      });
    }

    if (onSplitHorizontal || onSplitVertical) {
      items.push({ id: "divider-split", type: "divider" });
    }

    // 放大/缩小
    if (onToggleMagnify) {
      items.push({
        id: "magnify",
        label: isMagnified ? "取消放大" : "放大面板",
        icon: isMagnified ? <Minimize2 /> : <Maximize2 />,
        onClick: () => {
          onToggleMagnify();
          onClose();
        },
      });
    }

    // 清屏
    items.push({
      id: "clear",
      label: "清屏",
      icon: <Trash2 />,
      shortcut: "⌘K",
      onClick: handleClear,
    });

    // 重启终端
    if (onRestart) {
      items.push({
        id: "restart",
        label: "重启终端",
        icon: <RotateCcw />,
        onClick: () => {
          onRestart();
          onClose();
        },
      });
    }

    items.push({ id: "divider-2", type: "divider" });

    // 字体大小子菜单
    items.push({
      id: "font-size",
      label: "字体大小",
      icon: <Type />,
      subItems: FONT_SIZES.map((size) => ({
        id: `font-${size}`,
        label: `${size}px`,
        checked: currentFontSize === size,
        onClick: () => {
          onFontSizeChange(size);
          onClose();
        },
      })),
    });

    // 主题子菜单
    items.push({
      id: "theme",
      label: "主题",
      icon: <Palette />,
      subItems: AVAILABLE_THEMES.map((theme) => ({
        id: `theme-${theme.id}`,
        label: theme.name,
        checked: currentTheme === theme.id,
        onClick: () => {
          onThemeChange(theme.id);
          onClose();
        },
      })),
    });

    return items;
  }, [
    hasSelection,
    selectionUrl,
    currentFontSize,
    currentTheme,
    isMagnified,
    onSplitHorizontal,
    onSplitVertical,
    onToggleMagnify,
    onRestart,
    handleCopy,
    handlePaste,
    handleClear,
    handleOpenUrl,
    onFontSizeChange,
    onThemeChange,
    onClose,
  ]);

  // 渲染子菜单
  const renderSubMenu = (items: MenuItem[]) => {
    if (!subMenuPosition) return null;

    return (
      <SubMenuContainer
        style={{
          top: subMenuPosition.top,
          left: subMenuPosition.left,
          position: "fixed",
        }}
      >
        {items.map((item) => (
          <MenuItemButton
            key={item.id}
            onClick={item.onClick}
            $disabled={item.disabled}
          >
            <CheckMark>{item.checked ? "✓" : ""}</CheckMark>
            <MenuItemLabel>{item.label}</MenuItemLabel>
          </MenuItemButton>
        ))}
      </SubMenuContainer>
    );
  };

  return (
    <MenuContainer
      ref={menuRef}
      style={{
        left: position.x,
        top: position.y,
      }}
      onMouseLeave={() => setActiveSubMenu(null)}
    >
      {menuItems.map((item) =>
        "type" in item && item.type === "divider" ? (
          <MenuDividerLine key={item.id} />
        ) : (
          <div key={item.id}>
            <MenuItemButton
              $danger={(item as MenuItem).danger}
              $disabled={(item as MenuItem).disabled}
              onClick={
                (item as MenuItem).subItems
                  ? undefined
                  : (item as MenuItem).onClick
              }
              disabled={(item as MenuItem).disabled}
              onMouseEnter={
                (item as MenuItem).subItems
                  ? (e) => handleSubMenuHover(item.id, e)
                  : () => setActiveSubMenu(null)
              }
            >
              {(item as MenuItem).icon}
              <MenuItemLabel>{(item as MenuItem).label}</MenuItemLabel>
              {(item as MenuItem).shortcut && (
                <MenuItemShortcut>
                  {(item as MenuItem).shortcut}
                </MenuItemShortcut>
              )}
              {(item as MenuItem).subItems && <ChevronRight size={14} />}
            </MenuItemButton>
            {activeSubMenu === item.id &&
              (item as MenuItem).subItems &&
              renderSubMenu((item as MenuItem).subItems!)}
          </div>
        ),
      )}
    </MenuContainer>
  );
});

export default TerminalContextMenu;
