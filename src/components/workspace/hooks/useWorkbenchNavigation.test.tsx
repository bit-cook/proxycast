import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type UseWorkbenchNavigationParams,
  useWorkbenchNavigation,
} from "./useWorkbenchNavigation";
import {
  cleanupMountedRoots,
  clickElement,
  mountHarness,
  setupReactActEnvironment,
  type MountedRoot,
} from "./testUtils";

type NavigationHarnessProps = UseWorkbenchNavigationParams;

function NavigationHarness(props: NavigationHarnessProps) {
  const navigation = useWorkbenchNavigation(props);

  return (
    <div
      data-mode={navigation.workspaceMode}
      data-view={navigation.activeWorkspaceView}
      data-drawer={navigation.activeRightDrawer ?? "none"}
      data-show-workflow-rail={String(navigation.showWorkflowRail)}
      data-show-chat={String(navigation.showChatPanel)}
      data-should-render-left-sidebar={String(navigation.shouldRenderLeftSidebar)}
      data-is-create-view={String(navigation.isCreateWorkspaceView)}
      data-right-rail={String(navigation.shouldRenderWorkspaceRightRail)}
      data-view-label={navigation.activeWorkspaceViewLabel}
    >
      <button
        data-testid="apply-project-detail"
        onClick={() => navigation.applyInitialNavigationState("project-detail")}
      />
      <button
        data-testid="open-workflow"
        onClick={navigation.handleOpenWorkflowView}
      />
      <button
        data-testid="back-project-management"
        onClick={navigation.handleBackToProjectManagement}
      />
      <button
        data-testid="enter-publish"
        onClick={() => navigation.handleEnterWorkspaceView("publish")}
      />
      <button
        data-testid="switch-publish"
        onClick={() => navigation.handleSwitchWorkspaceView("publish")}
      />
      <button
        data-testid="switch-create"
        onClick={() => navigation.handleSwitchWorkspaceView("create")}
      />
      <button
        data-testid="prepare-tools-state"
        onClick={() => {
          navigation.setActiveRightDrawer("tools");
          navigation.setShowWorkflowRail(true);
        }}
      />
    </div>
  );
}

const mountedRoots: MountedRoot[] = [];

function createHarnessProps(
  overrides: Partial<NavigationHarnessProps> = {},
): NavigationHarnessProps {
  return {
    initialViewMode: "workspace",
    initialContentId: "content-1",
    defaultWorkspaceView: "create",
    navigationItems: [
      { key: "create", label: "创作" },
      { key: "workflow", label: "流程" },
      { key: "settings", label: "设置" },
    ],
    leftSidebarCollapsed: true,
    setLeftSidebarCollapsed: vi.fn(),
    isAgentChatWorkspace: true,
    hasPrimaryWorkspaceRenderer: false,
    ...overrides,
  };
}

function renderHarness(initialProps: Partial<NavigationHarnessProps> = {}) {
  return mountHarness(
    NavigationHarness,
    createHarnessProps(initialProps),
    mountedRoots,
  );
}

afterEach(() => {
  cleanupMountedRoots(mountedRoots);
});

beforeEach(() => {
  setupReactActEnvironment();
});

describe("useWorkbenchNavigation", () => {
  it("可按 project-detail 规则应用初始化导航状态", () => {
    const setLeftSidebarCollapsed = vi.fn();

    const { container } = renderHarness({
      initialViewMode: "project-management",
      initialContentId: undefined,
      leftSidebarCollapsed: false,
      setLeftSidebarCollapsed,
    });

    const applyButton = container.querySelector(
      "button[data-testid='apply-project-detail']",
    );
    expect(applyButton).not.toBeNull();

    clickElement(applyButton);

    const root = container.firstElementChild as HTMLElement | null;
    expect(root?.dataset.mode).toBe("workspace");
    expect(root?.dataset.view).toBe("workflow");
    expect(setLeftSidebarCollapsed).toHaveBeenLastCalledWith(true);
  });

  it("无 workflow 导航时打开流程动作会回退到 settings", () => {
    const setLeftSidebarCollapsed = vi.fn();

    const { container } = renderHarness({
      navigationItems: [
        { key: "create", label: "创作" },
        { key: "publish", label: "发布" },
        { key: "settings", label: "设置" },
      ],
      setLeftSidebarCollapsed,
    });

    const openWorkflowButton = container.querySelector(
      "button[data-testid='open-workflow']",
    );
    expect(openWorkflowButton).not.toBeNull();

    clickElement(openWorkflowButton);

    const root = container.firstElementChild as HTMLElement | null;
    expect(root?.dataset.view).toBe("settings");
    expect(root?.dataset.viewLabel).toBe("设置");
  });

  it("切换到非 create 视图时收起工具抽屉与流程轨", () => {
    const setLeftSidebarCollapsed = vi.fn();

    const { container } = renderHarness({
      navigationItems: [
        { key: "create", label: "创作" },
        { key: "publish", label: "发布" },
        { key: "settings", label: "设置" },
      ],
      setLeftSidebarCollapsed,
    });

    const prepareButton = container.querySelector(
      "button[data-testid='prepare-tools-state']",
    );
    expect(prepareButton).not.toBeNull();
    clickElement(prepareButton);

    let root = container.firstElementChild as HTMLElement | null;
    expect(root?.dataset.drawer).toBe("tools");
    expect(root?.dataset.showWorkflowRail).toBe("true");

    const switchPublishButton = container.querySelector(
      "button[data-testid='switch-publish']",
    );
    expect(switchPublishButton).not.toBeNull();
    clickElement(switchPublishButton);

    root = container.firstElementChild as HTMLElement | null;
    expect(root?.dataset.drawer).toBe("none");
    expect(root?.dataset.showWorkflowRail).toBe("false");
  });

  it("返回项目管理时恢复项目管理模式并展开左栏", () => {
    const setLeftSidebarCollapsed = vi.fn();

    const { container } = renderHarness({
      navigationItems: [
        { key: "create", label: "创作" },
        { key: "workflow", label: "流程" },
      ],
      setLeftSidebarCollapsed,
    });

    const backButton = container.querySelector(
      "button[data-testid='back-project-management']",
    );
    expect(backButton).not.toBeNull();

    clickElement(backButton);

    const root = container.firstElementChild as HTMLElement | null;
    expect(root?.dataset.mode).toBe("project-management");
    expect(root?.dataset.showChat).toBe("true");
    expect(root?.dataset.shouldRenderLeftSidebar).toBe("true");
    expect(setLeftSidebarCollapsed).toHaveBeenLastCalledWith(false);
  });
});
