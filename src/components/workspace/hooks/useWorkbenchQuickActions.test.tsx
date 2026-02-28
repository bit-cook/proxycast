import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type UseWorkbenchQuickActionsParams,
  useWorkbenchQuickActions,
} from "./useWorkbenchQuickActions";
import {
  cleanupMountedRoots,
  clickElement,
  mountHarness,
  setupReactActEnvironment,
  type MountedRoot,
} from "./testUtils";

type QuickActionsHarnessProps = UseWorkbenchQuickActionsParams;

function QuickActionsHarness(props: QuickActionsHarnessProps) {
  const { nonCreateQuickActions } = useWorkbenchQuickActions(props);

  return (
    <div>
      <div data-testid="action-count">{nonCreateQuickActions.length}</div>
      <div data-testid="action-keys">
        {nonCreateQuickActions.map((action) => action.key).join(",")}
      </div>
      <div data-testid="action-labels">
        {nonCreateQuickActions.map((action) => action.label).join("|")}
      </div>
      {nonCreateQuickActions.map((action) => (
        <button key={action.key} data-key={action.key} onClick={action.onClick}>
          {action.label}
        </button>
      ))}
    </div>
  );
}

const mountedRoots: MountedRoot[] = [];

function createHarnessProps(
  overrides: Partial<QuickActionsHarnessProps> = {},
): QuickActionsHarnessProps {
  return {
    workspaceMode: "workspace",
    activeWorkspaceView: "publish",
    hasWorkflowWorkspaceView: true,
    hasPublishWorkspaceView: true,
    hasSettingsWorkspaceView: true,
    selectedContentId: "content-1",
    onSwitchWorkspaceView: vi.fn(),
    onQuickSaveCurrent: vi.fn(),
    ...overrides,
  };
}

function renderHarness(initialProps: Partial<QuickActionsHarnessProps> = {}) {
  return mountHarness(
    QuickActionsHarness,
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

describe("useWorkbenchQuickActions", () => {
  it("非工作区或创作视图时不返回动作", () => {
    const onSwitchWorkspaceView = vi.fn();
    const onQuickSaveCurrent = vi.fn();

    const { container, rerender } = renderHarness({
      workspaceMode: "project-management",
      onSwitchWorkspaceView,
      onQuickSaveCurrent,
    });

    expect(
      container.querySelector("[data-testid='action-count']")?.textContent,
    ).toBe("0");

    rerender(
      createHarnessProps({
        workspaceMode: "workspace",
        activeWorkspaceView: "create",
        onSwitchWorkspaceView,
        onQuickSaveCurrent,
      }),
    );

    expect(
      container.querySelector("[data-testid='action-count']")?.textContent,
    ).toBe("0");
  });

  it("发布视图返回正确动作，并可触发回调", () => {
    const onSwitchWorkspaceView = vi.fn();
    const onQuickSaveCurrent = vi.fn();

    const { container } = renderHarness({
      onSwitchWorkspaceView,
      onQuickSaveCurrent,
    });

    const labels =
      container.querySelector("[data-testid='action-labels']")?.textContent ?? "";
    expect(labels).toContain("返回创作视图");
    expect(labels).toContain("前往流程视图");
    expect(labels).toContain("前往设置视图");
    expect(labels).toContain("快速保存当前文稿");
    expect(labels).not.toContain("前往发布视图");

    const workflowButton = container.querySelector(
      "button[data-key='to-workflow']",
    );
    expect(workflowButton).not.toBeNull();
    clickElement(workflowButton);
    expect(onSwitchWorkspaceView).toHaveBeenCalledWith("workflow");

    const saveButton = container.querySelector("button[data-key='quick-save']");
    expect(saveButton).not.toBeNull();
    clickElement(saveButton);
    expect(onQuickSaveCurrent).toHaveBeenCalledTimes(1);
  });

  it("在流程视图时包含前往发布动作", () => {
    const onSwitchWorkspaceView = vi.fn();
    const onQuickSaveCurrent = vi.fn();

    const { container } = renderHarness({
      activeWorkspaceView: "workflow",
      selectedContentId: null,
      onSwitchWorkspaceView,
      onQuickSaveCurrent,
    });

    const labels =
      container.querySelector("[data-testid='action-labels']")?.textContent ?? "";
    expect(labels).toContain("前往发布视图");
    expect(labels).not.toContain("快速保存当前文稿");
  });
});
