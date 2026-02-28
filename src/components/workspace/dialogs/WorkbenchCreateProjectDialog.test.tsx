import type { ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WorkbenchCreateProjectDialog } from "./WorkbenchCreateProjectDialog";
import {
  clickButtonByText,
  findButtonByText,
  cleanupMountedRoots,
  findInputById,
  fillTextInput,
  mountHarness,
  setupReactActEnvironment,
  type MountedRoot,
} from "../hooks/testUtils";

const mountedRoots: MountedRoot[] = [];

type ProjectDialogProps = ComponentProps<typeof WorkbenchCreateProjectDialog>;

function createDialogProps(
  overrides: Partial<ProjectDialogProps> = {},
): ProjectDialogProps {
  return {
    open: true,
    creatingProject: false,
    newProjectName: "小说项目A",
    projectTypeLabel: "小说创作",
    workspaceProjectsRoot: "/tmp/workspace",
    resolvedProjectPath: "/tmp/workspace/小说项目A",
    pathChecking: false,
    pathConflictMessage: "",
    onOpenChange: () => {},
    onProjectNameChange: () => {},
    onCreateProject: () => {},
    ...overrides,
  };
}

function renderDialog(
  overrides: Partial<ProjectDialogProps> = {},
) {
  return mountHarness(
    WorkbenchCreateProjectDialog,
    createDialogProps(overrides),
    mountedRoots,
  );
}

beforeEach(() => {
  setupReactActEnvironment();
});

afterEach(() => {
  cleanupMountedRoots(mountedRoots);
});

describe("WorkbenchCreateProjectDialog", () => {
  it("展示项目创建信息并支持编辑名称", () => {
    const onProjectNameChange = vi.fn();
    renderDialog({ onProjectNameChange });

    expect(document.body.textContent).toContain("新建项目");
    expect(document.body.textContent).toContain("/tmp/workspace/小说项目A");

    const projectTypeInput = findInputById(
      document.body,
      "workspace-project-type",
    ) as HTMLInputElement | null;
    expect(projectTypeInput?.value).toBe("小说创作");

    const projectNameInput = findInputById(
      document.body,
      "workspace-project-name",
    ) as HTMLInputElement | null;
    expect(projectNameInput).not.toBeNull();
    fillTextInput(projectNameInput, "小说项目B");

    expect(onProjectNameChange).toHaveBeenCalledWith("小说项目B");
  });

  it("路径冲突时禁用创建按钮", () => {
    renderDialog({ pathConflictMessage: "路径已存在项目：冲突项目" });

    const createButton = findButtonByText(document.body, "创建项目", {
      exact: true,
    });
    expect(createButton).toBeDefined();
    expect(createButton).toHaveProperty("disabled", true);
    expect(document.body.textContent).toContain("路径已存在项目：冲突项目");
  });

  it("点击取消与创建应触发对应回调", () => {
    const onOpenChange = vi.fn();
    const onCreateProject = vi.fn();
    renderDialog({ onOpenChange, onCreateProject });

    const cancelButton = findButtonByText(document.body, "取消", { exact: true });
    const createButton = findButtonByText(document.body, "创建项目", {
      exact: true,
    });
    expect(cancelButton).toBeDefined();
    expect(createButton).toBeDefined();

    clickButtonByText(document.body, "取消", { exact: true });
    clickButtonByText(document.body, "创建项目", { exact: true });

    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onCreateProject).toHaveBeenCalledTimes(1);
  });
});
