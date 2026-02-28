import { act, type ComponentProps } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useWorkbenchStore } from "@/stores/useWorkbenchStore";

const {
  mockListProjects,
  mockListContents,
  mockGetContent,
  mockCreateProject,
  mockCreateContent,
  mockUpdateContent,
} = vi.hoisted(() => ({
  mockListProjects: vi.fn(),
  mockListContents: vi.fn(),
  mockGetContent: vi.fn(),
  mockCreateProject: vi.fn(),
  mockCreateContent: vi.fn(),
  mockUpdateContent: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/components/agent", () => ({
  AgentChatPage: ({ hideTopBar }: { hideTopBar?: boolean }) => (
    <div data-testid="agent-chat-page" data-hide-topbar={String(hideTopBar)} />
  ),
}));

vi.mock("@/components/content-creator/canvas/video", () => ({
  VideoCanvas: ({ projectId }: { projectId?: string | null }) => (
    <div data-testid="video-canvas">video:{projectId ?? "none"}</div>
  ),
  createInitialVideoState: () => ({
    type: "video",
    prompt: "",
    providerId: "",
    model: "",
    duration: 5,
    generateAudio: false,
    cameraFixed: false,
    aspectRatio: "adaptive",
    resolution: "720p",
    status: "idle",
  }),
}));

vi.mock("@/lib/api/project", () => ({
  listProjects: mockListProjects,
  listContents: mockListContents,
  getContent: mockGetContent,
  createProject: mockCreateProject,
  createContent: mockCreateContent,
  updateContent: mockUpdateContent,
  getWorkspaceProjectsRoot: vi.fn(async () => "/tmp/workspace"),
  getProjectByRootPath: vi.fn(async () => null),
  resolveProjectRootPath: vi.fn(async (name: string) => `/tmp/workspace/${name}`),
  getCreateProjectErrorMessage: vi.fn((message: string) => message),
  extractErrorMessage: vi.fn(() => "mock-error"),
  formatRelativeTime: vi.fn(() => "刚刚"),
  getContentTypeLabel: vi.fn(() => "文稿"),
  getDefaultContentTypeForProject: vi.fn(() => "post"),
  getProjectTypeLabel: vi.fn((theme: string) =>
    theme === "social-media" ? "社媒内容" : theme,
  ),
}));

import { WorkbenchPage } from "./WorkbenchPage";

interface RenderResult {
  container: HTMLDivElement;
  root: Root;
}

const mountedRoots: Array<{ container: HTMLDivElement; root: Root }> = [];

function renderPage(
  props: Partial<ComponentProps<typeof WorkbenchPage>> = {},
): RenderResult {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  act(() => {
    root.render(<WorkbenchPage theme="social-media" {...props} />);
  });

  mountedRoots.push({ container, root });
  return { container, root };
}

async function flushEffects(times = 3): Promise<void> {
  for (let i = 0; i < times; i += 1) {
    await act(async () => {
      await Promise.resolve();
    });
  }
}

function getLeftSidebar(container: HTMLElement): HTMLElement | null {
  const matched = Array.from(container.querySelectorAll("aside")).find((aside) =>
    aside.className.includes("bg-muted/20"),
  );
  return (matched as HTMLElement | undefined) ?? null;
}

beforeEach(() => {
  (
    globalThis as typeof globalThis & {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;

  localStorage.clear();
  vi.clearAllMocks();
  useWorkbenchStore.getState().setLeftSidebarCollapsed(true);

  mockListProjects.mockResolvedValue([
    {
      id: "project-1",
      name: "社媒项目A",
      workspaceType: "social-media",
      rootPath: "/tmp/workspace/project-1",
      isDefault: false,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      isFavorite: false,
      isArchived: false,
      tags: [],
    },
  ]);

  mockListContents.mockResolvedValue([
    {
      id: "content-1",
      project_id: "project-1",
      title: "文稿A",
      content_type: "post",
      status: "draft",
      order: 0,
      word_count: 0,
      created_at: Date.now(),
      updated_at: Date.now(),
    },
  ]);

  mockGetContent.mockResolvedValue({
    id: "content-1",
    metadata: { creationMode: "guided" },
  });
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
  localStorage.clear();
});

describe("WorkbenchPage 左侧栏模式行为", () => {
  it("项目管理模式默认展开左侧栏", async () => {
    const { container } = renderPage({ viewMode: "project-management" });
    await flushEffects();

    const leftSidebar = getLeftSidebar(container);
    expect(leftSidebar).not.toBeNull();
    expect(leftSidebar?.className).toContain("w-[260px]");
    expect(container.textContent).toContain("主题项目管理");
  });

  it("项目管理模式点击项目后直接进入统一工作区", async () => {
    const { container } = renderPage({ viewMode: "project-management" });
    await flushEffects();

    const projectButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("社媒项目A"),
    );
    expect(projectButton).toBeDefined();

    act(() => {
      projectButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects();

    expect(container.querySelector("[data-testid='agent-chat-page']")).not.toBeNull();
    expect(container.textContent).toContain("创作");
    expect(container.textContent).toContain("发布");
  });

  it("作业模式默认收起左侧栏", async () => {
    const { container } = renderPage({
      viewMode: "workspace",
      projectId: "project-1",
      contentId: "content-1",
    });
    await flushEffects();

    expect(getLeftSidebar(container)).toBeNull();
    expect(container.textContent).not.toContain("主题项目管理");
  });

  it("作业模式展开侧栏后点击项目保持在统一工作区", async () => {
    const { container } = renderPage({
      viewMode: "workspace",
      projectId: "project-1",
      contentId: "content-1",
    });
    await flushEffects();

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "b",
          ctrlKey: true,
          bubbles: true,
        }),
      );
    });
    await flushEffects();

    const projectButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("社媒项目A"),
    );
    expect(projectButton).toBeDefined();

    act(() => {
      projectButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects();

    expect(container.querySelector("[data-testid='agent-chat-page']")).not.toBeNull();
  });

  it("工作区点击项目管理后回到项目管理态", async () => {
    const { container } = renderPage({
      viewMode: "workspace",
      projectId: "project-1",
      contentId: "content-1",
    });
    await flushEffects();

    const managementButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("项目管理"),
    );
    expect(managementButton).toBeDefined();

    act(() => {
      managementButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects();

    expect(container.textContent).toContain("统一创作工作区");
    expect(container.textContent).toContain("进入创作");
  });

  it("工作区点击项目管理后自动展开左侧栏", async () => {
    const { container } = renderPage({
      viewMode: "workspace",
      projectId: "project-1",
      contentId: "content-1",
    });
    await flushEffects();

    const managementButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("项目管理"),
    );
    expect(managementButton).not.toBeUndefined();

    act(() => {
      managementButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects();

    const leftSidebar = getLeftSidebar(container);
    expect(leftSidebar).not.toBeNull();
    expect(leftSidebar?.className).toContain("w-[260px]");
    expect(container.textContent).toContain("主题项目管理");
  });

  it("统一工作区中的聊天页隐藏内部顶部栏，避免双导航", async () => {
    const { container } = renderPage({
      viewMode: "workspace",
      projectId: "project-1",
      contentId: "content-1",
    });
    await flushEffects();

    const chat = container.querySelector("[data-testid='agent-chat-page']");
    expect(chat).not.toBeNull();
    expect(chat?.getAttribute("data-hide-topbar")).toBe("true");
  });

  it("视频主题在作业模式渲染主题工作区而非对话工作区", async () => {
    mockListProjects.mockResolvedValueOnce([
      {
        id: "video-project-1",
        name: "视频项目A",
        workspaceType: "video",
        rootPath: "/tmp/workspace/video-project-1",
        isDefault: false,
        createdAt: Date.now(),
        updatedAt: Date.now(),
        isFavorite: false,
        isArchived: false,
        tags: [],
      },
    ]);

    const { container } = renderPage({
      theme: "video",
      viewMode: "workspace",
      projectId: "video-project-1",
    });
    await flushEffects();

    expect(
      container.querySelector("[data-testid='video-theme-workspace']"),
    ).not.toBeNull();
    expect(container.querySelector("[data-testid='video-canvas']")).not.toBeNull();
    expect(container.querySelector("[data-testid='agent-chat-page']")).toBeNull();
  });

  it("切换到非创作视图时左侧显示紧凑提示并可返回创作视图", async () => {
    const { container } = renderPage({
      viewMode: "workspace",
      projectId: "project-1",
      contentId: "content-1",
    });
    await flushEffects();

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "b",
          ctrlKey: true,
          bubbles: true,
        }),
      );
    });
    await flushEffects();

    const publishButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "发布",
    );
    expect(publishButton).toBeDefined();

    act(() => {
      publishButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects();

    expect(container.textContent).toContain("当前处于「发布」视图");
    expect(container.textContent).toContain("当前文稿：文稿A");
    expect(container.textContent).toContain("返回创作视图");
    expect(container.querySelector("input[placeholder='搜索文稿...']")).toBeNull();

    const openViewActionsButton = container.querySelector(
      "button[title='展开视图动作']",
    );
    expect(openViewActionsButton).not.toBeNull();

    act(() => {
      openViewActionsButton?.dispatchEvent(
        new MouseEvent("click", { bubbles: true }),
      );
    });
    await flushEffects();

    expect(container.textContent).toContain("视图动作");
    expect(container.textContent).toContain("前往设置视图");

    const backToCreateButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "返回创作视图",
    );
    expect(backToCreateButton).toBeDefined();

    act(() => {
      backToCreateButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects();

    expect(container.querySelector("input[placeholder='搜索文稿...']")).not.toBeNull();
  });

  it("创建项目后保持选中新项目且重置项目搜索", async () => {
    mockListProjects
      .mockResolvedValueOnce([
        {
          id: "project-1",
          name: "社媒项目A",
          workspaceType: "social-media",
          rootPath: "/tmp/workspace/project-1",
          isDefault: false,
          createdAt: Date.now(),
          updatedAt: Date.now(),
          isFavorite: false,
          isArchived: false,
          tags: [],
        },
      ])
      .mockResolvedValueOnce([
        {
          id: "project-1",
          name: "社媒项目A",
          workspaceType: "social-media",
          rootPath: "/tmp/workspace/project-1",
          isDefault: false,
          createdAt: Date.now(),
          updatedAt: Date.now(),
          isFavorite: false,
          isArchived: false,
          tags: [],
        },
        {
          id: "project-2",
          name: "新项目B",
          workspaceType: "social-media",
          rootPath: "/tmp/workspace/新项目B",
          isDefault: false,
          createdAt: Date.now(),
          updatedAt: Date.now(),
          isFavorite: false,
          isArchived: false,
          tags: [],
        },
      ]);
    mockCreateProject.mockResolvedValue({
      id: "project-2",
      name: "新项目B",
      workspaceType: "social-media",
      rootPath: "/tmp/workspace/新项目B",
      isDefault: false,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      isFavorite: false,
      isArchived: false,
      tags: [],
    });

    const { container } = renderPage({
      viewMode: "workspace",
      projectId: "project-1",
      contentId: "content-1",
    });
    await flushEffects();

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "b",
          ctrlKey: true,
          bubbles: true,
        }),
      );
    });
    await flushEffects();

    const projectSearchInput = container.querySelector(
      "input[placeholder='搜索项目...']",
    ) as HTMLInputElement | null;
    expect(projectSearchInput).not.toBeNull();
    act(() => {
      if (!projectSearchInput) {
        return;
      }
      projectSearchInput.value = "关键字";
      projectSearchInput.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await flushEffects();
    expect(projectSearchInput?.value).toBe("关键字");

    const createProjectButton = container.querySelector("button[title='新建项目']");
    expect(createProjectButton).not.toBeNull();
    act(() => {
      createProjectButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects();

    const projectNameInput = document.querySelector(
      "#workspace-project-name",
    ) as HTMLInputElement | null;
    expect(projectNameInput).not.toBeNull();
    act(() => {
      if (!projectNameInput) {
        return;
      }
      projectNameInput.value = "新项目B";
      projectNameInput.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await flushEffects();

    const createButton = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "创建项目",
    );
    expect(createButton).toBeDefined();
    act(() => {
      createButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects(5);

    expect(mockCreateProject).toHaveBeenCalled();
    expect(mockListContents).toHaveBeenCalledWith("project-2");
    expect(projectSearchInput?.value).toBe("");

    const newProjectEntry = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("新项目B"),
    );
    const oldProjectEntry = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("社媒项目A"),
    );
    expect(newProjectEntry).toBeDefined();
    expect(newProjectEntry?.className).toContain("bg-accent text-accent-foreground");
    expect(oldProjectEntry).toBeDefined();
    expect(oldProjectEntry?.className).not.toContain(
      "bg-accent text-accent-foreground",
    );
  });
});
