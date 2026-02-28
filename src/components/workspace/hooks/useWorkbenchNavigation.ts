import { useCallback, useEffect, useState } from "react";
import type { WorkflowProgressSnapshot } from "@/components/agent/chat";
import type {
  ThemeWorkspaceNavigationItem,
  ThemeWorkspaceView,
} from "@/features/themes/types";
import type { WorkspaceViewMode } from "@/types/page";

function resolveInitialWorkspaceState(
  initialViewMode: WorkspaceViewMode | undefined,
  initialContentId: string | undefined,
  navigationItems: ThemeWorkspaceNavigationItem[],
  defaultWorkspaceView: ThemeWorkspaceView,
): {
  mode: WorkspaceViewMode;
  view: ThemeWorkspaceView;
} {
  const mode: WorkspaceViewMode =
    initialViewMode === "project-management"
      ? "project-management"
      : initialViewMode === "project-detail"
        ? "workspace"
        : initialViewMode ?? (initialContentId ? "workspace" : "project-management");

  const view: ThemeWorkspaceView =
    initialViewMode === "project-detail"
      ? navigationItems.some((item) => item.key === "workflow")
        ? "workflow"
        : "settings"
      : defaultWorkspaceView;

  return { mode, view };
}

export interface UseWorkbenchNavigationParams {
  initialViewMode?: WorkspaceViewMode;
  initialContentId?: string;
  defaultWorkspaceView: ThemeWorkspaceView;
  navigationItems: ThemeWorkspaceNavigationItem[];
  leftSidebarCollapsed: boolean;
  setLeftSidebarCollapsed: (collapsed: boolean) => void;
  isAgentChatWorkspace: boolean;
  hasPrimaryWorkspaceRenderer: boolean;
}

export function useWorkbenchNavigation({
  initialViewMode,
  initialContentId,
  defaultWorkspaceView,
  navigationItems,
  leftSidebarCollapsed,
  setLeftSidebarCollapsed,
  isAgentChatWorkspace,
  hasPrimaryWorkspaceRenderer,
}: UseWorkbenchNavigationParams) {
  const [activeRightDrawer, setActiveRightDrawer] = useState<"tools" | null>(
    null,
  );
  const [showChatPanel, setShowChatPanel] = useState(true);
  const [workflowProgress, setWorkflowProgress] =
    useState<WorkflowProgressSnapshot | null>(null);
  const [showWorkflowRail, setShowWorkflowRail] = useState(false);
  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceViewMode>(
    initialViewMode ?? (initialContentId ? "workspace" : "project-management"),
  );
  const [activeWorkspaceView, setActiveWorkspaceView] =
    useState<ThemeWorkspaceView>(defaultWorkspaceView);

  const applyInitialNavigationState = useCallback(
    (nextInitialViewMode?: WorkspaceViewMode, nextInitialContentId?: string) => {
      const { mode, view } = resolveInitialWorkspaceState(
        nextInitialViewMode,
        nextInitialContentId,
        navigationItems,
        defaultWorkspaceView,
      );

      setWorkspaceMode(mode);
      setActiveWorkspaceView(view);
      setShowChatPanel(true);
      setActiveRightDrawer(null);
      setLeftSidebarCollapsed(mode === "workspace");
    },
    [defaultWorkspaceView, navigationItems, setLeftSidebarCollapsed],
  );

  const handleOpenWorkflowView = useCallback(() => {
    const hasWorkflow = navigationItems.some((item) => item.key === "workflow");
    if (hasWorkflow) {
      setActiveWorkspaceView("workflow");
      return;
    }
    setActiveWorkspaceView("settings");
  }, [navigationItems]);

  const handleBackToProjectManagement = useCallback(() => {
    setWorkspaceMode("project-management");
    setShowChatPanel(true);
    setActiveRightDrawer(null);
    setLeftSidebarCollapsed(false);
  }, [setLeftSidebarCollapsed]);

  const handleEnterWorkspaceView = useCallback(
    (view: ThemeWorkspaceView) => {
      setWorkspaceMode("workspace");
      setActiveWorkspaceView(view);
      setActiveRightDrawer(null);
      setLeftSidebarCollapsed(true);
      if (view === "create") {
        setShowChatPanel(true);
      }
    },
    [setLeftSidebarCollapsed],
  );

  const handleSwitchWorkspaceView = useCallback((view: ThemeWorkspaceView) => {
    setActiveWorkspaceView(view);
    if (view === "create") {
      setShowChatPanel(true);
    } else {
      setActiveRightDrawer(null);
      setShowWorkflowRail(false);
    }
  }, []);

  useEffect(() => {
    if (workspaceMode !== "workspace") {
      setWorkflowProgress(null);
      setShowWorkflowRail(false);
    }
  }, [workspaceMode]);

  useEffect(() => {
    if (!workflowProgress || workflowProgress.steps.length === 0) {
      setShowWorkflowRail(false);
    }
  }, [workflowProgress]);

  useEffect(() => {
    const hasCurrentView = navigationItems.some(
      (item) => item.key === activeWorkspaceView,
    );
    if (hasCurrentView) {
      return;
    }
    setActiveWorkspaceView(defaultWorkspaceView);
  }, [activeWorkspaceView, defaultWorkspaceView, navigationItems]);

  const shouldRenderLeftSidebar =
    workspaceMode !== "workspace" || !leftSidebarCollapsed;
  const isCreateWorkspaceView =
    workspaceMode === "workspace" && activeWorkspaceView === "create";
  const shouldRenderWorkspaceRightRail =
    workspaceMode === "workspace" &&
    isAgentChatWorkspace &&
    !hasPrimaryWorkspaceRenderer;
  const activeWorkspaceViewLabel =
    navigationItems.find((item) => item.key === activeWorkspaceView)?.label ??
    "当前视图";
  const hasWorkflowWorkspaceView = navigationItems.some(
    (item) => item.key === "workflow",
  );
  const hasPublishWorkspaceView = navigationItems.some(
    (item) => item.key === "publish",
  );
  const hasSettingsWorkspaceView = navigationItems.some(
    (item) => item.key === "settings",
  );

  return {
    activeRightDrawer,
    setActiveRightDrawer,
    showChatPanel,
    setShowChatPanel,
    workflowProgress,
    setWorkflowProgress,
    showWorkflowRail,
    setShowWorkflowRail,
    workspaceMode,
    setWorkspaceMode,
    activeWorkspaceView,
    setActiveWorkspaceView,
    shouldRenderLeftSidebar,
    isCreateWorkspaceView,
    shouldRenderWorkspaceRightRail,
    activeWorkspaceViewLabel,
    hasWorkflowWorkspaceView,
    hasPublishWorkspaceView,
    hasSettingsWorkspaceView,
    applyInitialNavigationState,
    handleOpenWorkflowView,
    handleBackToProjectManagement,
    handleEnterWorkspaceView,
    handleSwitchWorkspaceView,
  };
}

export default useWorkbenchNavigation;
