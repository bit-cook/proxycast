/**
 * 应用主入口组件
 *
 * 管理页面路由和全局状态
 * 支持静态页面和动态插件页面路由
 * 包含启动画面和全局图标侧边栏
 *
 * _需求: 2.2, 3.2, 5.2_
 */

import React, { useState, useEffect, useCallback } from "react";
import styled from "styled-components";
import { withI18nPatch } from "./i18n/withI18nPatch";
import { SplashScreen } from "./components/SplashScreen";
import { AppSidebar } from "./components/AppSidebar";
// import { SettingsPage } from "./components/settings";
import { SettingsPageV2 } from "./components/settings-v2";
import { ApiServerPage } from "./components/api-server/ApiServerPage";
import { ProviderPoolPage } from "./components/provider-pool";
import { ToolsPage } from "./components/tools/ToolsPage";
import { AgentChatPage } from "./components/agent";
import { PluginsPage } from "./components/plugins/PluginsPage";
import { McpPanel } from "./components/mcp";
import { ImageGenPage } from "./components/image-gen";
import { ProjectsPage } from "./components/projects";
import { ProjectDetailPage } from "./components/projects/ProjectDetailPage";
import { CreateProjectDialog } from "./components/projects/CreateProjectDialog";
import { ProjectType } from "./lib/api/project";

import {
  TerminalWorkspace,
  SysinfoView,
  FileBrowserView,
  WebView,
} from "./components/terminal";
import { OnboardingWizard, useOnboardingState } from "./components/onboarding";
import { ConnectConfirmDialog } from "./components/connect";
import { showRegistryLoadError } from "./lib/utils/connectError";
import { useDeepLink } from "./hooks/useDeepLink";
import { useRelayRegistry } from "./hooks/useRelayRegistry";
import { ComponentDebugProvider } from "./contexts/ComponentDebugContext";
import { SoundProvider } from "./contexts/SoundProvider";
import { ComponentDebugOverlay } from "./components/dev";
import {
  Page,
  PageParams,
  AgentPageParams,
  ProjectDetailPageParams,
} from "./types/page";
import { open } from "@tauri-apps/plugin-dialog";
import { createProject, createContent } from "./lib/api/project";
import { toast } from "sonner";

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

const PageWrapper = styled.div<{ $isActive: boolean }>`
  flex: 1;
  padding: 24px;
  overflow: auto;
  display: ${(props) => (props.$isActive ? "block" : "none")};
`;

/**
 * 全屏页面容器（无 padding）
 * 用于终端等需要全屏显示的插件
 */
const FullscreenWrapper = styled.div<{ $isActive: boolean }>`
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: ${(props) => (props.$isActive ? "flex" : "none")};
  flex-direction: column;
  position: relative;
`;

function AppContent() {
  const [showSplash, setShowSplash] = useState(true);
  const [currentPage, setCurrentPage] = useState<Page>("agent");
  const [pageParams, setPageParams] = useState<PageParams>({});
  const { needsOnboarding, completeOnboarding } = useOnboardingState();

  // 推荐标签引导创建项目相关状态
  const [projectDialogOpen, setProjectDialogOpen] = useState(false);
  const [pendingRecommendation, setPendingRecommendation] = useState<{
    shortLabel: string;
    fullPrompt: string;
    projectType: ProjectType;
    projectName: string;
  } | null>(null);

  // 带参数的页面导航
  const handleNavigate = useCallback((page: Page, params?: PageParams) => {
    setCurrentPage(page);
    if (params) {
      setPageParams(params);
    } else {
      setPageParams({});
    }
  }, []);

  // 推荐标签点击处理 - 打开创建项目对话框
  const _handleRequestRecommendation = useCallback(
    (shortLabel: string, fullPrompt: string, currentTheme: string) => {
      // 主题标签映射
      const themeLabels: Record<string, string> = {
        "social-media": "社媒",
        poster: "海报",
        music: "音乐",
        knowledge: "知识",
        planning: "计划",
        novel: "小说",
        document: "文档",
        video: "视频",
        general: "对话",
      };

      const prefix = themeLabels[currentTheme] || "项目";
      const projectName = `${prefix}：${shortLabel}`;

      setPendingRecommendation({
        shortLabel,
        fullPrompt,
        projectType: currentTheme as ProjectType,
        projectName,
      });
      setProjectDialogOpen(true);
    },
    [],
  );

  // 创建项目并创建初始内容
  const handleCreateProjectFromRecommendation = async (
    name: string,
    type: ProjectType,
  ) => {
    // 选择项目目录
    const selectedPath = await open({
      directory: true,
      title: "选择项目目录",
    });

    if (!selectedPath) {
      throw new Error("用户取消选择目录");
    }

    const projectPath = Array.isArray(selectedPath)
      ? selectedPath.length === 1
        ? selectedPath[0]
        : null
      : selectedPath;

    if (!projectPath) {
      throw new Error("请选择单个项目目录");
    }

    // 创建项目
    const project = await createProject({
      name,
      rootPath: projectPath,
      workspaceType: type,
    });

    // 如果有待处理的推荐内容，创建初始 Content
    if (pendingRecommendation) {
      const content = await createContent({
        project_id: project.id,
        title: name,
        body: pendingRecommendation.fullPrompt,
      });

      // 导航到 Agent 页面
      handleNavigate("agent", {
        projectId: project.id,
        contentId: content.id,
      });

      // 清除待处理的推荐
      setPendingRecommendation(null);
    } else {
      // 没有初始内容，直接导航到项目页面
      handleNavigate("projects");
    }

    toast.success("项目创建成功");
  };

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
   * 渲染所有页面（保持挂载状态）
   *
   * 所有页面组件都会被渲染，但只有当前页面可见
   * 这样可以保持页面状态，避免切换时重置
   *
   * _需求: 2.2, 3.2_
   */
  const renderAllPages = () => {
    return (
      <>
        {/* Provider Pool 页面 */}
        <PageWrapper $isActive={currentPage === "provider-pool"}>
          <ProviderPoolPage />
        </PageWrapper>

        {/* 图片生成页面 */}
        <div
          style={{
            flex: 1,
            minHeight: 0,
            display: currentPage === "image-gen" ? "flex" : "none",
            flexDirection: "column",
          }}
        >
          <ImageGenPage onNavigate={setCurrentPage} />
        </div>

        {/* API Server 页面 */}
        <PageWrapper $isActive={currentPage === "api-server"}>
          <ApiServerPage />
        </PageWrapper>

        {/* Agent 页面 */}
        <div
          style={{
            flex: 1,
            minHeight: 0,
            display: currentPage === "agent" ? "flex" : "none",
            flexDirection: "column",
          }}
        >
          <AgentChatPage
            projectId={(pageParams as AgentPageParams).projectId}
            contentId={(pageParams as AgentPageParams).contentId}
          />
        </div>

        {/* 项目页面 */}
        <PageWrapper $isActive={currentPage === "projects"}>
          <ProjectsPage onNavigate={handleNavigate} />
        </PageWrapper>

        {/* 项目详情页 */}
        {currentPage === "project-detail" &&
          (pageParams as ProjectDetailPageParams).projectId && (
            <PageWrapper $isActive={true}>
              <ProjectDetailPage
                projectId={(pageParams as ProjectDetailPageParams).projectId}
                onBack={() => handleNavigate("projects")}
                onNavigateToChat={(contentId) => {
                  handleNavigate("agent", {
                    projectId: (pageParams as ProjectDetailPageParams)
                      .projectId,
                    contentId,
                  });
                }}
              />
            </PageWrapper>
          )}

        {/* 终端工作区 - 使用 div 包装以支持显示/隐藏 */}
        <div
          style={{
            flex: 1,
            minHeight: 0,
            display: currentPage === "terminal" ? "flex" : "none",
            flexDirection: "column",
          }}
        >
          <TerminalWorkspace onNavigate={setCurrentPage} />
        </div>

        {/* 系统监控页面 */}
        <FullscreenWrapper $isActive={currentPage === "sysinfo"}>
          <SysinfoView />
        </FullscreenWrapper>

        {/* 文件浏览器页面 */}
        <FullscreenWrapper $isActive={currentPage === "files"}>
          <FileBrowserView />
        </FullscreenWrapper>

        {/* 内嵌浏览器页面 */}
        <FullscreenWrapper $isActive={currentPage === "web"}>
          <WebView />
        </FullscreenWrapper>

        {/* Tools 页面 */}
        <PageWrapper $isActive={currentPage === "tools"}>
          <ToolsPage onNavigate={setCurrentPage} />
        </PageWrapper>

        {/* MCP 页面 */}
        <PageWrapper $isActive={currentPage === "mcp"}>
          <McpPanel />
        </PageWrapper>

        {/* Plugins 页面 */}
        <PageWrapper $isActive={currentPage === "plugins"}>
          <PluginsPage />
        </PageWrapper>

        {/* Settings 页面 - 使用新版 V2 布局 */}
        <div
          style={{
            flex: 1,
            minHeight: 0,
            display: currentPage === "settings" ? "flex" : "none",
            flexDirection: "column",
          }}
        >
          <SettingsPageV2 />
        </div>

        {/* 动态插件页面已移除 */}
      </>
    );
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
    <SoundProvider>
      <ComponentDebugProvider>
        <AppContainer>
          <AppSidebar currentPage={currentPage} onNavigate={handleNavigate} />
          <MainContent>{renderAllPages()}</MainContent>
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
          {/* 创建项目对话框 - 用于推荐标签引导创建 */}
          <CreateProjectDialog
            open={projectDialogOpen}
            onOpenChange={(open) => {
              setProjectDialogOpen(open);
              if (!open) {
                // 用户取消，清除待处理的推荐
                setPendingRecommendation(null);
              }
            }}
            onSubmit={handleCreateProjectFromRecommendation}
            defaultType={pendingRecommendation?.projectType}
            defaultName={pendingRecommendation?.projectName}
          />
          {/* 组件视图调试覆盖层 */}
          <ComponentDebugOverlay />
        </AppContainer>
      </ComponentDebugProvider>
    </SoundProvider>
  );
}

// Export the App component wrapped with i18n patch support
const App = withI18nPatch(AppContent);
export default App;
