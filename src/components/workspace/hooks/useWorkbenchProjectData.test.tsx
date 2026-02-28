import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type UseWorkbenchProjectDataParams,
  useWorkbenchProjectData,
} from "./useWorkbenchProjectData";
import {
  cleanupMountedRoots,
  clickByTestId,
  flushEffects,
  getRootElement,
  mountHarness,
  setupReactActEnvironment,
  type MountedRoot,
} from "./testUtils";
import {
  createWorkspaceContentFixture,
  createWorkspaceProjectFixture,
} from "../testFixtures";

const { mockListContents, mockListProjects, mockToastError } = vi.hoisted(() => ({
  mockListContents: vi.fn(),
  mockListProjects: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    error: mockToastError,
  },
}));

vi.mock("@/lib/api/project", () => ({
  listProjects: mockListProjects,
  listContents: mockListContents,
}));

type HarnessProps = UseWorkbenchProjectDataParams;

function WorkbenchProjectDataHarness(props: HarnessProps) {
  const data = useWorkbenchProjectData(props);

  return (
    <div
      data-selected-project-id={data.selectedProjectId ?? ""}
      data-selected-content-id={data.selectedContentId ?? ""}
      data-project-query={data.projectQuery}
      data-content-query={data.contentQuery}
      data-project-ids={data.projects.map((item) => item.id).join(",")}
      data-filtered-project-ids={data.filteredProjects.map((item) => item.id).join(",")}
      data-content-ids={data.contents.map((item) => item.id).join(",")}
      data-filtered-content-ids={data.filteredContents.map((item) => item.id).join(",")}
    >
      <button
        data-testid="load-projects"
        onClick={() => {
          void data.loadProjects();
        }}
      />
      <button
        data-testid="load-contents-project-a"
        onClick={() => {
          void data.loadContents("project-a");
        }}
      />
      <button
        data-testid="set-selected-project-manual"
        onClick={() => data.setSelectedProjectId("project-manual")}
      />
      <button
        data-testid="set-selected-content-manual"
        onClick={() => data.setSelectedContentId("content-manual")}
      />
      <button
        data-testid="set-project-query-manual"
        onClick={() => data.setProjectQuery("manual")}
      />
      <button
        data-testid="set-content-query-manual"
        onClick={() => data.setContentQuery("manual")}
      />
      <button
        data-testid="reset-queries"
        onClick={data.resetProjectAndContentQueries}
      />
      <button
        data-testid="clear-contents-selection"
        onClick={data.clearContentsSelection}
      />
    </div>
  );
}

const mountedRoots: MountedRoot[] = [];

function renderHarness(props: Partial<HarnessProps> = {}) {
  const baseProps: HarnessProps = {
    theme: "social-media",
    initialProjectId: undefined,
    initialContentId: undefined,
  };

  return mountHarness(
    WorkbenchProjectDataHarness,
    { ...baseProps, ...props },
    mountedRoots,
  );
}

function click(container: HTMLElement, testId: string): void {
  const button = clickByTestId(container, testId);
  expect(button).not.toBeNull();
}

beforeEach(() => {
  setupReactActEnvironment();

  vi.clearAllMocks();
});

afterEach(() => {
  cleanupMountedRoots(mountedRoots);
});

describe("useWorkbenchProjectData", () => {
  it("loadProjects 仅保留当前主题且未归档项目，并使用 initialProjectId", async () => {
    mockListProjects.mockResolvedValue([
      createWorkspaceProjectFixture({
        id: "project-init",
        name: "初始化项目",
        workspaceType: "social-media",
        rootPath: "/tmp/workspace/project-init",
        isArchived: false,
        tags: [],
      }),
      createWorkspaceProjectFixture({
        id: "project-other-theme",
        name: "视频项目",
        workspaceType: "video",
        rootPath: "/tmp/workspace/project-other-theme",
        isArchived: false,
        tags: [],
      }),
      createWorkspaceProjectFixture({
        id: "project-archived",
        name: "归档项目",
        workspaceType: "social-media",
        rootPath: "/tmp/workspace/project-archived",
        isArchived: true,
        tags: [],
      }),
      createWorkspaceProjectFixture({
        id: "project-manual",
        name: "Manual 标签项目",
        workspaceType: "social-media",
        rootPath: "/tmp/workspace/project-manual",
        isArchived: false,
        tags: ["manual"],
      }),
    ]);

    const { container } = renderHarness({
      initialProjectId: "project-init",
    });

    click(container, "load-projects");
    await flushEffects();

    const root = getRootElement(container);
    expect(root?.dataset.projectIds).toBe("project-init,project-manual");
    expect(root?.dataset.selectedProjectId).toBe("project-init");
  });

  it("项目选择优先级应为 previousId > initialProjectId", async () => {
    mockListProjects.mockResolvedValue([
      createWorkspaceProjectFixture({
        id: "project-init",
        name: "初始化项目",
        workspaceType: "social-media",
        rootPath: "/tmp/workspace/project-init",
        isArchived: false,
        tags: [],
      }),
      createWorkspaceProjectFixture({
        id: "project-manual",
        name: "Manual 标签项目",
        workspaceType: "social-media",
        rootPath: "/tmp/workspace/project-manual",
        isArchived: false,
        tags: ["manual"],
      }),
    ]);

    const { container } = renderHarness({
      initialProjectId: "project-init",
    });

    click(container, "load-projects");
    await flushEffects();
    click(container, "set-selected-project-manual");
    await flushEffects();
    click(container, "load-projects");
    await flushEffects();
    click(container, "set-project-query-manual");
    await flushEffects();

    const root = getRootElement(container);
    expect(root?.dataset.selectedProjectId).toBe("project-manual");
    expect(root?.dataset.filteredProjectIds).toBe("project-manual");
  });

  it("文稿选择优先级应为 previousId > initialContentId，且支持筛选", async () => {
    mockListContents.mockResolvedValue([
      createWorkspaceContentFixture({
        id: "content-init",
        project_id: "project-a",
        title: "初始化文稿",
      }),
      createWorkspaceContentFixture({
        id: "content-manual",
        project_id: "project-a",
        title: "manual 文稿",
      }),
    ]);

    const { container } = renderHarness({
      initialContentId: "content-init",
    });

    click(container, "load-contents-project-a");
    await flushEffects();
    click(container, "set-selected-content-manual");
    await flushEffects();
    click(container, "load-contents-project-a");
    await flushEffects();
    click(container, "set-content-query-manual");
    await flushEffects();

    const root = getRootElement(container);
    expect(root?.dataset.selectedContentId).toBe("content-manual");
    expect(root?.dataset.filteredContentIds).toBe("content-manual");
  });

  it("应支持重置查询与清空文稿选择", async () => {
    mockListContents.mockResolvedValue([
      createWorkspaceContentFixture({
        id: "content-manual",
        project_id: "project-a",
        title: "manual 文稿",
      }),
    ]);

    const { container } = renderHarness();

    click(container, "load-contents-project-a");
    await flushEffects();
    click(container, "set-selected-content-manual");
    click(container, "set-project-query-manual");
    click(container, "set-content-query-manual");
    await flushEffects();

    click(container, "reset-queries");
    click(container, "clear-contents-selection");
    await flushEffects();

    const root = getRootElement(container);
    expect(root?.dataset.projectQuery).toBe("");
    expect(root?.dataset.contentQuery).toBe("");
    expect(root?.dataset.selectedContentId).toBe("");
    expect(root?.dataset.contentIds).toBe("");
  });
});
