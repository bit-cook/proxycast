import { act, type ComponentProps } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WorkbenchCreateProjectDialog } from "./WorkbenchCreateProjectDialog";

interface RenderResult {
  container: HTMLDivElement;
  root: Root;
}

const mountedRoots: RenderResult[] = [];

function setInputValue(input: HTMLInputElement, value: string): void {
  const setter = Object.getOwnPropertyDescriptor(
    window.HTMLInputElement.prototype,
    "value",
  )?.set;
  setter?.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function renderDialog(
  overrides: Partial<ComponentProps<typeof WorkbenchCreateProjectDialog>> = {},
): RenderResult {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  act(() => {
    root.render(
      <WorkbenchCreateProjectDialog
        open={true}
        creatingProject={false}
        newProjectName="小说项目A"
        projectTypeLabel="小说创作"
        workspaceProjectsRoot="/tmp/workspace"
        resolvedProjectPath="/tmp/workspace/小说项目A"
        pathChecking={false}
        pathConflictMessage=""
        onOpenChange={() => {}}
        onProjectNameChange={() => {}}
        onCreateProject={() => {}}
        {...overrides}
      />,
    );
  });

  const rendered = { container, root };
  mountedRoots.push(rendered);
  return rendered;
}

beforeEach(() => {
  (
    globalThis as typeof globalThis & {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;
});

afterEach(() => {
  while (mountedRoots.length > 0) {
    const mounted = mountedRoots.pop();
    if (!mounted) {
      break;
    }
    act(() => {
      mounted.root.unmount();
    });
    mounted.container.remove();
  }
});

describe("WorkbenchCreateProjectDialog", () => {
  it("展示项目创建信息并支持编辑名称", () => {
    const onProjectNameChange = vi.fn();
    renderDialog({ onProjectNameChange });

    expect(document.body.textContent).toContain("新建项目");
    expect(document.body.textContent).toContain("/tmp/workspace/小说项目A");

    const projectTypeInput = document.body.querySelector(
      "input#workspace-project-type",
    ) as HTMLInputElement | null;
    expect(projectTypeInput?.value).toBe("小说创作");

    const projectNameInput = document.body.querySelector(
      "input#workspace-project-name",
    ) as HTMLInputElement | null;
    expect(projectNameInput).not.toBeNull();

    act(() => {
      if (!projectNameInput) {
        return;
      }
      setInputValue(projectNameInput, "小说项目B");
    });

    expect(onProjectNameChange).toHaveBeenCalledWith("小说项目B");
  });

  it("路径冲突时禁用创建按钮", () => {
    renderDialog({ pathConflictMessage: "路径已存在项目：冲突项目" });

    const createButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "创建项目",
    );
    expect(createButton).toBeDefined();
    expect(createButton).toHaveProperty("disabled", true);
    expect(document.body.textContent).toContain("路径已存在项目：冲突项目");
  });

  it("点击取消与创建应触发对应回调", () => {
    const onOpenChange = vi.fn();
    const onCreateProject = vi.fn();
    renderDialog({ onOpenChange, onCreateProject });

    const cancelButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "取消",
    );
    const createButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "创建项目",
    );
    expect(cancelButton).toBeDefined();
    expect(createButton).toBeDefined();

    act(() => {
      cancelButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    act(() => {
      createButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onCreateProject).toHaveBeenCalledTimes(1);
  });
});
