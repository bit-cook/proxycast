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
  AgentChatPage: ({
    onBackToProjectManagement,
  }: {
    onBackToProjectManagement?: () => void;
  }) => (
    <div data-testid="agent-chat-page">
      <button
        type="button"
        onClick={() => {
          onBackToProjectManagement?.();
        }}
      >
        从聊天返回项目管理
      </button>
    </div>
  ),
}));

vi.mock("@/components/projects/ProjectDetailPage", () => ({
  ProjectDetailPage: ({
    onBack,
    onNavigateToChat,
  }: {
    onBack?: () => void;
    onNavigateToChat?: () => void;
  }) => (
    <div data-testid="project-detail-page">
      <button
        type="button"
        onClick={() => {
          onBack?.();
        }}
      >
        返回项目管理
      </button>
      <button
        type="button"
        onClick={() => {
          onNavigateToChat?.();
        }}
      >
        进入作业
      </button>
    </div>
  ),
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

  it("项目详情模式默认展开左侧栏", async () => {
    const { container } = renderPage({
      viewMode: "project-detail",
      projectId: "project-1",
    });
    await flushEffects();

    const leftSidebar = getLeftSidebar(container);
    expect(leftSidebar).not.toBeNull();
    expect(leftSidebar?.className).toContain("w-[260px]");
    expect(container.textContent).toContain("主题项目管理");
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

  it("从作业返回项目管理后自动展开左侧栏", async () => {
    const { container } = renderPage({
      viewMode: "workspace",
      projectId: "project-1",
      contentId: "content-1",
    });
    await flushEffects();

    const backButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("从聊天返回项目管理"),
    );
    expect(backButton).not.toBeUndefined();

    act(() => {
      backButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await flushEffects();

    const leftSidebar = getLeftSidebar(container);
    expect(leftSidebar).not.toBeNull();
    expect(leftSidebar?.className).toContain("w-[260px]");
    expect(container.textContent).toContain("主题项目管理");
  });
});
