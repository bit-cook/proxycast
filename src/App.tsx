/**
 * 应用主入口组件
 *
 * 管理页面路由和全局状态
 * 支持静态页面和动态插件页面路由
 * 包含启动画面和全局图标侧边栏
 *
 * _需求: 2.2, 3.2, 5.2_
 */

import { useState, useEffect, useCallback } from "react";
import styled from "styled-components";
import { withI18nPatch } from "./i18n/withI18nPatch";
import { SplashScreen } from "./components/SplashScreen";
import { AppSidebar } from "./components/AppSidebar";
import { SettingsPage } from "./components/settings";
import { ApiServerPage } from "./components/api-server/ApiServerPage";
import { ProviderPoolPage } from "./components/provider-pool";
import { ToolsPage } from "./components/tools/ToolsPage";
import { AgentChatPage } from "./components/agent";
import { PluginUIRenderer } from "./components/plugins/PluginUIRenderer";
import { PluginsPage } from "./components/plugins/PluginsPage";
import {
  TerminalWorkspace,
  SysinfoView,
  FileBrowserView,
  WebView,
} from "./components/terminal";
import { flowEventManager } from "./lib/flowEventManager";
import { OnboardingWizard, useOnboardingState } from "./components/onboarding";
import { ConnectConfirmDialog } from "./components/connect";
import { showRegistryLoadError } from "./lib/utils/connectError";
import { useDeepLink } from "./hooks/useDeepLink";
import { useRelayRegistry } from "./hooks/useRelayRegistry";
import { ComponentDebugProvider } from "./contexts/ComponentDebugContext";
import { ComponentDebugOverlay } from "./components/dev";
import { Page } from "./types/page";

const AppContainer = styled.div`
  display: flex;
  height: 100vh;
  width: 100vw;
  background-color: hsl(var(--background));
  overflow: hidden;
`;

const MainContent = styled.main`
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
`;

const PageWrapper = styled.div`
  flex: 1;
  padding: 24px;
  overflow: auto;
`;

/**
 * 全屏页面容器（无 padding）
 * 用于终端等需要全屏显示的插件
 */
const FullscreenWrapper = styled.div`
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  position: relative;
`;

function AppContent() {
  const [showSplash, setShowSplash] = useState(true);
  const [currentPage, setCurrentPage] = useState<Page>("agent");
  const { needsOnboarding, completeOnboarding } = useOnboardingState();

  // Deep Link 处理 Hook
  // _Requirements: 5.2_
  const {
    connectPayload,
    relayInfo,
    isVerified,
    isDialogOpen,
    isSaving,
    error,
    handleConfirm,
    handleCancel,
  } = useDeepLink();

  // Relay Registry 管理 Hook
  // _Requirements: 2.1, 7.2, 7.3_
  const {
    error: registryError,
    refresh: _refreshRegistry, // 保留以供后续错误处理 UI 使用
  } = useRelayRegistry();

  // 在应用启动时初始化 Flow 事件订阅
  useEffect(() => {
    flowEventManager.subscribe();
  }, []);

  // 处理 Registry 加载失败
  // _Requirements: 7.2, 7.3_
  useEffect(() => {
    if (registryError) {
      console.warn("[App] Registry 加载失败:", registryError);
      // 显示 toast 通知用户
      showRegistryLoadError(registryError.message);
    }
  }, [registryError]);

  // 页面切换时重置滚动位置
  useEffect(() => {
    const mainElement = document.querySelector("main");
    if (mainElement) {
      mainElement.scrollTop = 0;
    }
  }, [currentPage]);

  const handleSplashComplete = useCallback(() => {
    setShowSplash(false);
  }, []);

  /**
   * 渲染当前页面
   *
   * 根据 currentPage 状态渲染对应的页面组件
   * - 静态页面: 直接渲染对应组件
   * - 动态插件页面: 使用 PluginUIRenderer 渲染
   *
   * _需求: 2.2, 3.2_
   */
  const renderPage = () => {
    // 检查是否为动态插件页面 (plugin:xxx 格式)
    if (currentPage.startsWith("plugin:")) {
      const pluginId = currentPage.slice(7); // 移除 "plugin:" 前缀

      // 需要全屏显示的插件列表
      const fullscreenPlugins = ["terminal-plugin"];
      const isFullscreen = fullscreenPlugins.includes(pluginId);

      if (isFullscreen) {
        return (
          <FullscreenWrapper>
            <PluginUIRenderer pluginId={pluginId} onNavigate={setCurrentPage} />
          </FullscreenWrapper>
        );
      }

      return (
        <PageWrapper>
          <PluginUIRenderer pluginId={pluginId} onNavigate={setCurrentPage} />
        </PageWrapper>
      );
    }

    // 静态页面路由
    switch (currentPage) {
      case "provider-pool":
        return (
          <PageWrapper>
            <ProviderPoolPage />
          </PageWrapper>
        );
      case "api-server":
        return (
          <PageWrapper>
            <ApiServerPage />
          </PageWrapper>
        );
      case "agent":
        // Agent 页面有自己的布局，不需要 PageWrapper
        return (
          <AgentChatPage onNavigate={(page) => setCurrentPage(page as Page)} />
        );
      case "terminal":
        // 终端工作区 - 分块布局（对齐 Waveterm TileLayout）
        return <TerminalWorkspace onNavigate={setCurrentPage} />;
      case "sysinfo":
        // 系统监控页面
        return (
          <FullscreenWrapper>
            <SysinfoView />
          </FullscreenWrapper>
        );
      case "files":
        // 文件浏览器页面
        return (
          <FullscreenWrapper>
            <FileBrowserView />
          </FullscreenWrapper>
        );
      case "web":
        // 内嵌浏览器页面
        return (
          <FullscreenWrapper>
            <WebView />
          </FullscreenWrapper>
        );
      case "tools":
        return (
          <PageWrapper>
            <ToolsPage onNavigate={setCurrentPage} />
          </PageWrapper>
        );
      case "plugins":
        return (
          <PageWrapper>
            <PluginsPage />
          </PageWrapper>
        );
      case "settings":
        return (
          <PageWrapper>
            <SettingsPage />
          </PageWrapper>
        );
      default:
        return (
          <PageWrapper>
            <ApiServerPage />
          </PageWrapper>
        );
    }
  };

  // 引导完成回调
  const handleOnboardingComplete = useCallback(() => {
    completeOnboarding();
  }, [completeOnboarding]);

  // 1. 显示启动画面
  if (showSplash) {
    return <SplashScreen onComplete={handleSplashComplete} />;
  }

  // 2. 检测中，显示空白
  if (needsOnboarding === null) {
    return null;
  }

  // 3. 需要引导时显示引导向导
  if (needsOnboarding) {
    return <OnboardingWizard onComplete={handleOnboardingComplete} />;
  }

  // 4. 正常主界面
  return (
    <ComponentDebugProvider>
      <AppContainer>
        <AppSidebar currentPage={currentPage} onNavigate={setCurrentPage} />
        <MainContent>{renderPage()}</MainContent>
        {/* ProxyCast Connect 确认弹窗 */}
        {/* _Requirements: 5.2_ */}
        <ConnectConfirmDialog
          open={isDialogOpen}
          relay={relayInfo}
          relayId={connectPayload?.relay ?? ""}
          apiKey={connectPayload?.key ?? ""}
          keyName={connectPayload?.name}
          isVerified={isVerified}
          isSaving={isSaving}
          error={error}
          onConfirm={handleConfirm}
          onCancel={handleCancel}
        />
        {/* 组件视图调试覆盖层 */}
        <ComponentDebugOverlay />
      </AppContainer>
    </ComponentDebugProvider>
  );
}

// Export the App component wrapped with i18n patch support
const App = withI18nPatch(AppContent);
export default App;
