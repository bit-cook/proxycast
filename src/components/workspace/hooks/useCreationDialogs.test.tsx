import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type UseCreationDialogsParams,
  useCreationDialogs,
} from "./useCreationDialogs";
import {
  cleanupMountedRoots,
  clickByTestId,
  flushEffects,
  getRootElement,
  mountHarness,
  setupReactActEnvironment,
  type MountedRoot,
} from "./testUtils";

const {
  mockCreateContent,
  mockCreateProject,
  mockExtractErrorMessage,
  mockGetContent,
  mockGetContentTypeLabel,
  mockGetCreateProjectErrorMessage,
  mockGetDefaultContentTypeForProject,
  mockGetProjectByRootPath,
  mockGetProjectTypeLabel,
  mockGetWorkspaceProjectsRoot,
  mockResolveProjectRootPath,
  mockToastError,
  mockToastSuccess,
} = vi.hoisted(() => ({
  mockCreateContent: vi.fn(),
  mockCreateProject: vi.fn(),
  mockExtractErrorMessage: vi.fn(),
  mockGetContent: vi.fn(),
  mockGetContentTypeLabel: vi.fn(),
  mockGetCreateProjectErrorMessage: vi.fn(),
  mockGetDefaultContentTypeForProject: vi.fn(),
  mockGetProjectByRootPath: vi.fn(),
  mockGetProjectTypeLabel: vi.fn(),
  mockGetWorkspaceProjectsRoot: vi.fn(),
  mockResolveProjectRootPath: vi.fn(),
  mockToastError: vi.fn(),
  mockToastSuccess: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: mockToastSuccess,
    error: mockToastError,
  },
}));

vi.mock("@/lib/api/project", () => ({
  createContent: mockCreateContent,
  createProject: mockCreateProject,
  extractErrorMessage: mockExtractErrorMessage,
  getContent: mockGetContent,
  getContentTypeLabel: mockGetContentTypeLabel,
  getCreateProjectErrorMessage: mockGetCreateProjectErrorMessage,
  getDefaultContentTypeForProject: mockGetDefaultContentTypeForProject,
  getProjectByRootPath: mockGetProjectByRootPath,
  getProjectTypeLabel: mockGetProjectTypeLabel,
  getWorkspaceProjectsRoot: mockGetWorkspaceProjectsRoot,
  resolveProjectRootPath: mockResolveProjectRootPath,
}));

type HarnessProps = UseCreationDialogsParams;

function CreationDialogsHarness(props: HarnessProps) {
  const dialogs = useCreationDialogs(props);

  return (
    <div
      data-create-project-open={String(dialogs.createProjectDialogOpen)}
      data-create-content-open={String(dialogs.createContentDialogOpen)}
      data-create-content-step={dialogs.createContentDialogStep}
      data-new-project-name={dialogs.newProjectName}
      data-workspace-root={dialogs.workspaceProjectsRoot}
      data-resolved-project-path={dialogs.resolvedProjectPath}
      data-path-checking={String(dialogs.pathChecking)}
      data-path-conflict-message={dialogs.pathConflictMessage}
      data-creating-project={String(dialogs.creatingProject)}
      data-creating-content={String(dialogs.creatingContent)}
      data-creation-intent-error={dialogs.creationIntentError}
      data-current-intent-length={String(dialogs.currentIntentLength)}
      data-pending-prompts={JSON.stringify(dialogs.pendingInitialPromptsByContentId)}
      data-content-modes={JSON.stringify(dialogs.contentCreationModes)}
    >
      <button
        data-testid="open-project-dialog"
        onClick={dialogs.handleOpenCreateProjectDialog}
      />
      <button
        data-testid="close-project-dialog"
        onClick={() => dialogs.setCreateProjectDialogOpen(false)}
      />
      <button
        data-testid="set-project-name-new"
        onClick={() => dialogs.setNewProjectName("新项目A")}
      />
      <button
        data-testid="set-project-name-conflict"
        onClick={() => dialogs.setNewProjectName("冲突项目")}
      />
      <button
        data-testid="create-project"
        onClick={() => {
          void dialogs.handleCreateProject();
        }}
      />

      <button
        data-testid="open-content-dialog"
        onClick={dialogs.handleOpenCreateContentDialog}
      />
      <button data-testid="goto-intent" onClick={dialogs.handleGoToIntentStep} />
      <button
        data-testid="fill-intent-topic"
        onClick={() =>
          dialogs.handleCreationIntentValueChange("topic", "这是一个足够详细的创作主题")
        }
      />
      <button
        data-testid="create-content"
        onClick={() => {
          void dialogs.handleCreateContent();
        }}
      />
      <button
        data-testid="consume-prompt"
        onClick={() => dialogs.consumePendingInitialPrompt("content-new")}
      />
    </div>
  );
}

const mountedRoots: MountedRoot[] = [];

function createHarnessProps(overrides: Partial<HarnessProps> = {}): HarnessProps {
  return {
    theme: "social-media",
    selectedProjectId: null,
    selectedContentId: null,
    loadProjects: vi.fn(async () => undefined),
    loadContents: vi.fn(async () => undefined),
    onEnterWorkspace: vi.fn(),
    onProjectCreated: vi.fn(),
    defaultCreationMode: "guided",
    minCreationIntentLength: 10,
    ...overrides,
  };
}

function renderHarness(props: Partial<HarnessProps> = {}) {
  return mountHarness(
    CreationDialogsHarness,
    createHarnessProps(props),
    mountedRoots,
  );
}

function click(container: HTMLElement, testId: string): void {
  const button = clickByTestId(container, testId);
  expect(button).not.toBeNull();
}

async function openContentIntentStep(container: HTMLElement): Promise<void> {
  click(container, "open-content-dialog");
  await flushEffects();
  click(container, "goto-intent");
  await flushEffects();
}

function parseRootDatasetRecord(
  root: HTMLElement | null,
  key: string,
): Record<string, string> {
  const value = root?.dataset[key] ?? "{}";
  return JSON.parse(value) as Record<string, string>;
}

beforeEach(() => {
  setupReactActEnvironment();

  vi.clearAllMocks();

  mockGetWorkspaceProjectsRoot.mockResolvedValue("/tmp/workspace");
  mockGetProjectTypeLabel.mockReturnValue("社媒内容");
  mockResolveProjectRootPath.mockImplementation(
    async (name: string) => `/tmp/workspace/${name}`,
  );
  mockGetProjectByRootPath.mockResolvedValue(null);
  mockCreateProject.mockResolvedValue({
    id: "project-new",
    name: "新项目A",
  });
  mockExtractErrorMessage.mockReturnValue("mock-error");
  mockGetCreateProjectErrorMessage.mockReturnValue("mock-friendly-error");

  mockGetDefaultContentTypeForProject.mockReturnValue("post");
  mockGetContentTypeLabel.mockReturnValue("文稿");
  mockCreateContent.mockResolvedValue({
    id: "content-new",
  });
  mockGetContent.mockResolvedValue(null);
});

afterEach(() => {
  cleanupMountedRoots(mountedRoots);
});

describe("useCreationDialogs", () => {
  it("创建项目成功后应关闭弹窗并触发回调", async () => {
    const loadProjects = vi.fn(async () => undefined);
    const onProjectCreated = vi.fn();

    const { container } = renderHarness({
      loadProjects,
      onProjectCreated,
    });
    await flushEffects();

    click(container, "open-project-dialog");
    await flushEffects();
    click(container, "set-project-name-new");
    await flushEffects();
    click(container, "create-project");
    await flushEffects(6);

    expect(mockCreateProject).toHaveBeenCalledWith({
      name: "新项目A",
      rootPath: "/tmp/workspace/新项目A",
      workspaceType: "social-media",
    });
    expect(onProjectCreated).toHaveBeenCalledWith("project-new");
    expect(loadProjects).toHaveBeenCalledTimes(1);
    expect(mockToastSuccess).toHaveBeenCalledWith("已创建新项目");

    const root = getRootElement(container);
    expect(root?.dataset.createProjectOpen).toBe("false");
    expect(root?.dataset.creatingProject).toBe("false");
  });

  it("打开项目弹窗后应检测路径冲突", async () => {
    mockGetProjectByRootPath.mockImplementation(async (rootPath: string) => {
      if (rootPath.endsWith("/冲突项目")) {
        return { id: "project-existing", name: "历史项目" };
      }
      return null;
    });

    const { container } = renderHarness();
    await flushEffects();

    click(container, "open-project-dialog");
    await flushEffects();
    click(container, "set-project-name-conflict");
    await flushEffects(6);

    const root = getRootElement(container);
    expect(root?.dataset.resolvedProjectPath).toBe("/tmp/workspace/冲突项目");
    expect(root?.dataset.pathChecking).toBe("false");
    expect(root?.dataset.pathConflictMessage).toBe("路径已存在项目：历史项目");
    expect(mockGetProjectByRootPath).toHaveBeenCalledWith("/tmp/workspace/冲突项目");
  });

  it("关闭项目弹窗后应重置路径状态", async () => {
    mockGetProjectByRootPath.mockImplementation(async (rootPath: string) => {
      if (rootPath.endsWith("/冲突项目")) {
        return { id: "project-existing", name: "历史项目" };
      }
      return null;
    });

    const { container } = renderHarness();
    await flushEffects();

    click(container, "open-project-dialog");
    await flushEffects();
    click(container, "set-project-name-conflict");
    await flushEffects(6);

    let root = getRootElement(container);
    expect(root?.dataset.pathConflictMessage).toBe("路径已存在项目：历史项目");
    expect(root?.dataset.resolvedProjectPath).toBe("/tmp/workspace/冲突项目");

    click(container, "close-project-dialog");
    await flushEffects();

    root = getRootElement(container);
    expect(root?.dataset.createProjectOpen).toBe("false");
    expect(root?.dataset.pathConflictMessage).toBe("");
    expect(root?.dataset.resolvedProjectPath).toBe("");
    expect(root?.dataset.pathChecking).toBe("false");
  });

  it("选中文稿且 metadata 含 creationMode 时应回填模式缓存", async () => {
    mockGetContent.mockResolvedValueOnce({
      id: "content-existing",
      metadata: {
        creationMode: "framework",
      },
    });

    const { container } = renderHarness({
      selectedContentId: "content-existing",
    });
    await flushEffects(5);

    expect(mockGetContent).toHaveBeenCalledWith("content-existing");
    const root = getRootElement(container);
    const contentModes = parseRootDatasetRecord(root, "contentModes");
    expect(contentModes["content-existing"]).toBe("framework");
  });

  it("创作意图不足时创建文稿应被阻止并提示错误", async () => {
    const { container } = renderHarness({
      selectedProjectId: "project-1",
    });
    await flushEffects();

    await openContentIntentStep(container);
    click(container, "create-content");
    await flushEffects();

    const root = getRootElement(container);
    expect(root?.dataset.createContentOpen).toBe("true");
    expect(root?.dataset.createContentStep).toBe("intent");
    expect(root?.dataset.creationIntentError).toContain("创作意图至少需要 10 个字");
    expect(mockCreateContent).not.toHaveBeenCalled();
  });

  it("创作意图通过后创建文稿应写入待发送提示并进入工作区", async () => {
    const loadContents = vi.fn(async () => undefined);
    const onEnterWorkspace = vi.fn();

    const { container } = renderHarness({
      selectedProjectId: "project-1",
      loadContents,
      onEnterWorkspace,
    });
    await flushEffects();

    await openContentIntentStep(container);
    click(container, "fill-intent-topic");
    await flushEffects();
    click(container, "create-content");
    await flushEffects(6);

    expect(mockCreateContent).toHaveBeenCalledTimes(1);
    expect(loadContents).toHaveBeenCalledWith("project-1");
    expect(onEnterWorkspace).toHaveBeenCalledWith("content-new", {
      showChatPanel: true,
    });
    expect(mockToastSuccess).toHaveBeenCalledWith("已创建新文稿");

    const root = getRootElement(container);
    expect(root?.dataset.createContentOpen).toBe("false");
    expect(root?.dataset.createContentStep).toBe("mode");

    const pendingPrompts = parseRootDatasetRecord(root, "pendingPrompts");
    const contentModes = parseRootDatasetRecord(root, "contentModes");
    expect(Object.keys(pendingPrompts)).toContain("content-new");
    expect(contentModes["content-new"]).toBe("guided");

    click(container, "consume-prompt");
    await flushEffects();
    const rootAfterConsume = getRootElement(container);
    const pendingPromptsAfterConsume = parseRootDatasetRecord(
      rootAfterConsume,
      "pendingPrompts",
    );
    expect(pendingPromptsAfterConsume["content-new"]).toBeUndefined();
  });
});
