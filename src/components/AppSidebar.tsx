/**
 * 全局应用侧边栏
 *
 * 类似 cherry-studio 的图标导航栏，始终显示在应用左侧
 */

import { useState, useEffect } from "react";
import styled from "styled-components";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Bot,
  Globe,
  Database,
  Wrench,
  Puzzle,
  Settings,
  Moon,
  Sun,
  Activity,
  Terminal,
  LucideIcon,
} from "lucide-react";
import * as LucideIcons from "lucide-react";
import { getPluginsForSurface, PluginUIInfo } from "@/lib/api/pluginUI";
import { Page } from "@/types/page";

interface AppSidebarProps {
  currentPage: Page;
  onNavigate: (page: Page) => void;
}

const Container = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 54px;
  min-width: 54px;
  height: 100vh;
  padding: 12px 0;
  background-color: hsl(var(--card));
  border-right: 1px solid hsl(var(--border));
`;

const LogoContainer = styled.div`
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 16px;
  cursor: pointer;
  transition: transform 0.2s;

  &:hover {
    transform: scale(1.05);
  }
`;

const LogoImg = styled.img`
  width: 32px;
  height: 32px;
  object-fit: contain;
`;

const MenusContainer = styled.div`
  display: flex;
  flex-direction: column;
  flex: 1;
  gap: 4px;
  overflow-y: auto;
  overflow-x: hidden;

  &::-webkit-scrollbar {
    display: none;
  }
`;

const BottomMenus = styled.div`
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: auto;
  padding-top: 8px;
  border-top: 1px solid hsl(var(--border));
`;

const IconButton = styled.button<{ $active?: boolean }>`
  width: 38px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  border: none;
  background: ${({ $active }) =>
    $active ? "hsl(var(--primary))" : "transparent"};
  color: ${({ $active }) =>
    $active
      ? "hsl(var(--primary-foreground))"
      : "hsl(var(--muted-foreground))"};
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    background: ${({ $active }) =>
      $active ? "hsl(var(--primary))" : "hsl(var(--muted))"};
    color: ${({ $active }) =>
      $active ? "hsl(var(--primary-foreground))" : "hsl(var(--foreground))"};
  }

  svg {
    width: 20px;
    height: 20px;
  }
`;

const mainMenuItems: { id: Page; label: string; icon: typeof Bot }[] = [
  { id: "agent", label: "AI Agent", icon: Bot },
  { id: "api-server", label: "API Server", icon: Globe },
  { id: "provider-pool", label: "凭证池", icon: Database },
  { id: "terminal", label: "终端", icon: Terminal },
  { id: "tools", label: "工具", icon: Wrench },
  { id: "plugins", label: "插件中心", icon: Puzzle },
];

/**
 * 根据图标名称获取 Lucide 图标组件
 * 默认返回 Activity 图标
 */
function getIconByName(iconName: string): LucideIcon {
  const IconComponent = (
    LucideIcons as unknown as Record<string, LucideIcon | undefined>
  )[iconName];
  return IconComponent || Activity;
}

export function AppSidebar({ currentPage, onNavigate }: AppSidebarProps) {
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    if (typeof window !== "undefined") {
      return document.documentElement.classList.contains("dark")
        ? "dark"
        : "light";
    }
    return "light";
  });

  // 已安装的侧边栏插件列表
  const [sidebarPlugins, setSidebarPlugins] = useState<PluginUIInfo[]>([]);
  // 刷新触发器
  const [refreshTrigger, setRefreshTrigger] = useState(0);

  // 加载侧边栏插件
  useEffect(() => {
    const loadSidebarPlugins = async () => {
      try {
        const plugins = await getPluginsForSurface("sidebar");
        setSidebarPlugins(plugins);
      } catch (error) {
        console.error("加载侧边栏插件失败:", error);
      }
    };
    loadSidebarPlugins();
  }, [refreshTrigger]);

  // 监听插件安装/卸载事件，刷新侧边栏
  useEffect(() => {
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === "plugin-changed") {
        setRefreshTrigger((prev) => prev + 1);
      }
    };

    // 监听自定义事件
    const handlePluginChange = () => {
      setRefreshTrigger((prev) => prev + 1);
    };

    window.addEventListener("storage", handleStorageChange);
    window.addEventListener("plugin-changed", handlePluginChange);

    return () => {
      window.removeEventListener("storage", handleStorageChange);
      window.removeEventListener("plugin-changed", handlePluginChange);
    };
  }, []);

  useEffect(() => {
    if (theme === "dark") {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
    localStorage.setItem("theme", theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme(theme === "dark" ? "light" : "dark");
  };

  return (
    <TooltipProvider>
      <Container>
        <Tooltip>
          <TooltipTrigger asChild>
            <LogoContainer onClick={() => onNavigate("agent")}>
              <LogoImg src="/logo.png" alt="ProxyCast" />
            </LogoContainer>
          </TooltipTrigger>
          <TooltipContent side="right">
            <span className="whitespace-nowrap">ProxyCast</span>
          </TooltipContent>
        </Tooltip>

        <MenusContainer>
          {mainMenuItems.map((item) => (
            <Tooltip key={item.id}>
              <TooltipTrigger asChild>
                <IconButton
                  $active={currentPage === item.id}
                  onClick={() => onNavigate(item.id)}
                >
                  <item.icon />
                </IconButton>
              </TooltipTrigger>
              <TooltipContent side="right">
                <span className="whitespace-nowrap">{item.label}</span>
              </TooltipContent>
            </Tooltip>
          ))}
          {/* 动态插件入口 */}
          {sidebarPlugins.map((plugin) => {
            const PluginIcon = getIconByName(plugin.icon);
            const pluginPageId: Page = `plugin:${plugin.pluginId}`;
            return (
              <Tooltip key={plugin.pluginId}>
                <TooltipTrigger asChild>
                  <IconButton
                    $active={currentPage === pluginPageId}
                    onClick={() => onNavigate(pluginPageId)}
                  >
                    <PluginIcon />
                  </IconButton>
                </TooltipTrigger>
                <TooltipContent side="right">
                  <span className="whitespace-nowrap">{plugin.name}</span>
                </TooltipContent>
              </Tooltip>
            );
          })}
        </MenusContainer>

        <BottomMenus>
          <Tooltip>
            <TooltipTrigger asChild>
              <IconButton onClick={toggleTheme}>
                {theme === "dark" ? <Moon /> : <Sun />}
              </IconButton>
            </TooltipTrigger>
            <TooltipContent side="right">
              <span className="whitespace-nowrap">
                {theme === "dark" ? "深色模式" : "浅色模式"}
              </span>
            </TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <IconButton
                $active={currentPage === "settings"}
                onClick={() => onNavigate("settings")}
              >
                <Settings />
              </IconButton>
            </TooltipTrigger>
            <TooltipContent side="right">
              <span className="whitespace-nowrap">设置</span>
            </TooltipContent>
          </Tooltip>
        </BottomMenus>
      </Container>
    </TooltipProvider>
  );
}
