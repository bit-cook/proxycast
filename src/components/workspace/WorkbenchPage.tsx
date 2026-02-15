/**
 * @file WorkbenchPage.tsx
 * @description 主题工作台页面，按主题管理项目并复用 Agent 对话与画布
 * @module components/workspace/WorkbenchPage
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Bot,
  FileText,
  FolderOpen,
  Home,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  RefreshCw,
  Sparkles,
  Wrench,
} from "lucide-react";
import { useWorkbenchStore } from "@/stores/useWorkbenchStore";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import {
  type ContentListItem,
  type Project,
  type ProjectType,
  createContent,
  createProject,
  formatRelativeTime,
  getContentTypeLabel,
  getDefaultContentTypeForProject,
  getProjectTypeLabel,
  getWorkspaceProjectsRoot,
  getProjectByRootPath,
  getCreateProjectErrorMessage,
  extractErrorMessage,
  resolveProjectRootPath,
  listContents,
  listProjects,
  getContent,
  updateContent,
} from "@/lib/api/project";
import type {
  Page,
  PageParams,
  WorkspaceTheme,
  WorkspaceViewMode,
} from "@/types/page";
import { toast } from "sonner";
import { AgentChatPage } from "@/components/agent";
import type { WorkflowProgressSnapshot } from "@/components/agent/chat";
import { buildHomeAgentParams } from "@/lib/workspace/navigation";
import { ProjectDetailPage } from "@/components/projects/ProjectDetailPage";
import type { CreationMode } from "@/components/content-creator/types";
import {
  buildCreationIntentMetadata,
  buildCreationIntentPrompt,
  createInitialCreationIntentValues,
  getCreationIntentFields,
  type CreationIntentFieldKey,
  type CreationIntentFormValues,
  type CreationIntentInput,
  validateCreationIntent,
} from "@/components/workspace/utils/creationIntentPrompt";

export interface WorkbenchPageProps {
  onNavigate?: (page: Page, params?: PageParams) => void;
  projectId?: string;
  contentId?: string;
  theme: WorkspaceTheme;
  viewMode?: WorkspaceViewMode;
  resetAt?: number;
}

type WorkspaceMode = WorkspaceViewMode;
type CreateContentDialogStep = "mode" | "intent";

const DEFAULT_CREATION_MODE: CreationMode = "guided";
const MIN_CREATION_INTENT_LENGTH = 10;

const CREATION_MODE_OPTIONS: Array<{
  value: CreationMode;
  label: string;
  description: string;
}> = [
  {
    value: "guided",
    label: "引导模式",
    description: "AI 分步骤提问引导，适合精细创作",
  },
  {
    value: "fast",
    label: "快速模式",
    description: "AI 先生成初稿，适合快速起稿",
  },
  {
    value: "hybrid",
    label: "混合模式",
    description: "AI 与你协作，平衡质量和效率",
  },
  {
    value: "framework",
    label: "框架模式",
    description: "你定结构，AI 按框架补全内容",
  },
];

function parseCreationMode(value: unknown): CreationMode | null {
  if (
    value === "guided" ||
    value === "fast" ||
    value === "hybrid" ||
    value === "framework"
  ) {
    return value;
  }
  return null;
}

function getWorkflowStepStatusLabel(
  status: WorkflowProgressSnapshot["steps"][number]["status"],
): string {
  switch (status) {
    case "active":
      return "进行中";
    case "completed":
      return "已完成";
    case "skipped":
      return "已跳过";
    case "error":
      return "异常";
    default:
      return "待开始";
  }
}

export function WorkbenchPage({
  onNavigate,
  projectId: initialProjectId,
  contentId: initialContentId,
  theme,
  viewMode: initialViewMode,
  resetAt,
}: WorkbenchPageProps) {
  const { leftSidebarCollapsed, toggleLeftSidebar, setLeftSidebarCollapsed } =
    useWorkbenchStore();
  const [activeRightDrawer, setActiveRightDrawer] = useState<"tools" | null>(
    null,
  );
  const [showChatPanel, setShowChatPanel] = useState(true);
  const [workflowProgress, setWorkflowProgress] =
    useState<WorkflowProgressSnapshot | null>(null);
  const [showWorkflowRail, setShowWorkflowRail] = useState(false);
  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceMode>(
    initialViewMode ?? (initialContentId ? "workspace" : "project-management"),
  );

  const [projects, setProjects] = useState<Project[]>([]);
  const [projectsLoading, setProjectsLoading] = useState(false);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(
    initialProjectId ?? null,
  );

  const [contents, setContents] = useState<ContentListItem[]>([]);
  const [contentsLoading, setContentsLoading] = useState(false);
  const [selectedContentId, setSelectedContentId] = useState<string | null>(
    initialContentId ?? null,
  );

  const [projectQuery, setProjectQuery] = useState("");
  const [contentQuery, setContentQuery] = useState("");

  const [createProjectDialogOpen, setCreateProjectDialogOpen] = useState(false);
  const [createContentDialogOpen, setCreateContentDialogOpen] = useState(false);
  const [createContentDialogStep, setCreateContentDialogStep] =
    useState<CreateContentDialogStep>("mode");
  const [newProjectName, setNewProjectName] = useState("");
  const [workspaceProjectsRoot, setWorkspaceProjectsRoot] = useState("");
  const [creatingProject, setCreatingProject] = useState(false);
  const [creatingContent, setCreatingContent] = useState(false);
  const [selectedCreationMode, setSelectedCreationMode] =
    useState<CreationMode>(DEFAULT_CREATION_MODE);
  const [creationIntentValues, setCreationIntentValues] =
    useState<CreationIntentFormValues>(() => createInitialCreationIntentValues());
  const [creationIntentError, setCreationIntentError] = useState("");
  const [pendingInitialPromptsByContentId, setPendingInitialPromptsByContentId] =
    useState<Record<string, string>>({});
  const [contentCreationModes, setContentCreationModes] = useState<
    Record<string, CreationMode>
  >({});
  const [resolvedProjectPath, setResolvedProjectPath] = useState("");
  const [pathChecking, setPathChecking] = useState(false);
  const [pathConflictMessage, setPathConflictMessage] = useState("");

  const selectedProject = useMemo(
    () => projects.find((project) => project.id === selectedProjectId) ?? null,
    [projects, selectedProjectId],
  );

  const filteredProjects = useMemo(() => {
    const query = projectQuery.trim().toLowerCase();
    if (!query) {
      return projects;
    }

    return projects.filter(
      (project) =>
        project.name.toLowerCase().includes(query) ||
        project.tags.some((tag) => tag.toLowerCase().includes(query)),
    );
  }, [projects, projectQuery]);

  const filteredContents = useMemo(() => {
    const query = contentQuery.trim().toLowerCase();
    if (!query) {
      return contents;
    }

    return contents.filter((content) =>
      content.title.toLowerCase().includes(query),
    );
  }, [contents, contentQuery]);

  const creationIntentInput = useMemo<CreationIntentInput>(
    () => ({
      creationMode: selectedCreationMode,
      values: creationIntentValues,
    }),
    [selectedCreationMode, creationIntentValues],
  );

  const currentCreationIntentFields = useMemo(
    () => getCreationIntentFields(selectedCreationMode),
    [selectedCreationMode],
  );

  const currentIntentLength = useMemo(
    () => validateCreationIntent(creationIntentInput, MIN_CREATION_INTENT_LENGTH)
      .length,
    [creationIntentInput],
  );

  const handleEnterWorkspace = useCallback(
    (
      contentId: string,
      options?: {
        showChatPanel?: boolean;
      },
    ) => {
      setSelectedContentId(contentId);
      setWorkspaceMode("workspace");
      setShowChatPanel(options?.showChatPanel ?? false);
      setActiveRightDrawer(null);
      setLeftSidebarCollapsed(true);
    },
    [setLeftSidebarCollapsed],
  );

  const handleOpenProjectDetail = useCallback(() => {
    if (!selectedProjectId) {
      return;
    }

    setWorkspaceMode("project-detail");
    setActiveRightDrawer(null);
    setLeftSidebarCollapsed(false);
  }, [selectedProjectId, setLeftSidebarCollapsed]);

  const loadProjects = useCallback(async () => {
    setProjectsLoading(true);
    try {
      const allProjects = await listProjects();
      const typedProjects = allProjects.filter(
        (project) =>
          project.workspaceType === (theme as ProjectType) &&
          !project.isArchived,
      );

      setProjects(typedProjects);
      setSelectedProjectId((previousId) => {
        if (
          initialProjectId &&
          typedProjects.some((project) => project.id === initialProjectId)
        ) {
          return initialProjectId;
        }

        if (
          previousId &&
          typedProjects.some((project) => project.id === previousId)
        ) {
          return previousId;
        }

        return typedProjects[0]?.id ?? null;
      });
    } catch (error) {
      console.error("加载主题项目失败:", error);
      toast.error("加载项目失败");
    } finally {
      setProjectsLoading(false);
    }
  }, [initialProjectId, theme]);

  const loadContents = useCallback(
    async (projectId: string) => {
      setContentsLoading(true);
      try {
        const contentList = await listContents(projectId);
        setContents(contentList);

        setSelectedContentId((previousId) => {
          if (
            initialContentId &&
            contentList.some((content) => content.id === initialContentId)
          ) {
            return initialContentId;
          }

          if (
            previousId &&
            contentList.some((content) => content.id === previousId)
          ) {
            return previousId;
          }

          return contentList[0]?.id ?? null;
        });
      } catch (error) {
        console.error("加载文稿失败:", error);
        toast.error("加载文稿失败");
      } finally {
        setContentsLoading(false);
      }
    },
    [initialContentId],
  );

  const handleOpenCreateProjectDialog = useCallback(() => {
    setNewProjectName(`${getProjectTypeLabel(theme as ProjectType)}项目`);
    setResolvedProjectPath("");
    setPathConflictMessage("");
    setPathChecking(false);
    setCreateProjectDialogOpen(true);
  }, [theme]);

  const handleCreateProject = useCallback(async () => {
    const name = newProjectName.trim();

    if (!name) {
      toast.error("请输入项目名称");
      return;
    }

    setCreatingProject(true);
    try {
      const rootPath = await resolveProjectRootPath(name);
      const createdProject = await createProject({
        name,
        rootPath,
        workspaceType: theme as ProjectType,
      });
      setCreateProjectDialogOpen(false);
      setSelectedProjectId(createdProject.id);
      setProjectQuery("");
      toast.success("已创建新项目");
      await loadProjects();
    } catch (error) {
      console.error("创建项目失败:", error);
      const errorMessage = extractErrorMessage(error);
      const friendlyMessage = getCreateProjectErrorMessage(errorMessage);
      toast.error(`创建项目失败: ${friendlyMessage}`);
    } finally {
      setCreatingProject(false);
    }
  }, [loadProjects, newProjectName, theme]);

  const resetCreateContentDialogState = useCallback(() => {
    setCreateContentDialogStep("mode");
    setSelectedCreationMode(DEFAULT_CREATION_MODE);
    setCreationIntentValues(createInitialCreationIntentValues());
    setCreationIntentError("");
  }, []);

  const handleOpenCreateContentDialog = useCallback(() => {
    if (!selectedProjectId) {
      return;
    }

    resetCreateContentDialogState();
    setCreateContentDialogOpen(true);
  }, [resetCreateContentDialogState, selectedProjectId]);

  const handleCreationIntentValueChange = useCallback(
    (key: CreationIntentFieldKey, value: string) => {
      setCreationIntentValues((previous) => ({
        ...previous,
        [key]: value,
      }));
      if (creationIntentError) {
        setCreationIntentError("");
      }
    },
    [creationIntentError],
  );

  const handleGoToIntentStep = useCallback(() => {
    setCreateContentDialogStep("intent");
    setCreationIntentError("");
  }, []);

  const handleCreateContent = useCallback(
    async () => {
      if (!selectedProjectId) {
        return;
      }

      const validation = validateCreationIntent(
        creationIntentInput,
        MIN_CREATION_INTENT_LENGTH,
      );
      if (!validation.valid) {
        setCreationIntentError(validation.message || "请完善创作意图");
        return;
      }

      const initialUserPrompt = buildCreationIntentPrompt(creationIntentInput);
      const creationIntentMetadata = buildCreationIntentMetadata(
        creationIntentInput,
      );

      setCreatingContent(true);
      try {
        const defaultType = getDefaultContentTypeForProject(
          theme as ProjectType,
        );
        const created = await createContent({
          project_id: selectedProjectId,
          title: `新${getContentTypeLabel(defaultType)}`,
          content_type: defaultType,
          metadata: {
            creationMode: selectedCreationMode,
            creationIntent: creationIntentMetadata,
          },
        });

        setContentCreationModes((previous) => ({
          ...previous,
          [created.id]: selectedCreationMode,
        }));
        setPendingInitialPromptsByContentId((previous) => ({
          ...previous,
          [created.id]: initialUserPrompt,
        }));
        setCreateContentDialogOpen(false);
        resetCreateContentDialogState();
        await loadContents(selectedProjectId);
        handleEnterWorkspace(created.id, { showChatPanel: true });
        toast.success("已创建新文稿");
      } catch (error) {
        console.error("创建文稿失败:", error);
        toast.error("创建文稿失败");
      } finally {
        setCreatingContent(false);
      }
    },
    [
      creationIntentInput,
      handleEnterWorkspace,
      loadContents,
      resetCreateContentDialogState,
      selectedCreationMode,
      selectedProjectId,
      theme,
    ],
  );

  const consumePendingInitialPrompt = useCallback((contentId: string) => {
    setPendingInitialPromptsByContentId((previous) => {
      if (!previous[contentId]) {
        return previous;
      }
      const next = { ...previous };
      delete next[contentId];
      return next;
    });
  }, []);

  const handleQuickSaveCurrent = useCallback(async () => {
    if (!selectedContentId || !selectedProjectId) {
      return;
    }

    try {
      await updateContent(selectedContentId, {
        metadata: {
          saved_from: "theme-workspace",
          saved_at: Date.now(),
        },
      });
      toast.success("已保存当前文稿");
      await loadContents(selectedProjectId);
    } catch (error) {
      console.error("保存失败:", error);
      toast.error("保存失败");
    }
  }, [loadContents, selectedContentId, selectedProjectId]);

  useEffect(() => {
    const nextMode: WorkspaceMode =
      initialViewMode ??
      (initialContentId ? "workspace" : "project-management");

    setProjectQuery("");
    setContentQuery("");
    setSelectedProjectId(initialProjectId ?? null);
    setSelectedContentId(initialContentId ?? null);
    setWorkspaceMode(nextMode);
    const isWorkspaceMode = nextMode === "workspace";
    setShowChatPanel(!isWorkspaceMode);
    setLeftSidebarCollapsed(isWorkspaceMode);
    setActiveRightDrawer(null);
    setContents([]);
    void loadProjects();
  }, [
    initialContentId,
    initialProjectId,
    initialViewMode,
    loadProjects,
    resetAt,
    setLeftSidebarCollapsed,
    theme,
  ]);

  useEffect(() => {
    let mounted = true;

    const loadWorkspaceProjectsRoot = async () => {
      try {
        const root = await getWorkspaceProjectsRoot();
        if (mounted) {
          setWorkspaceProjectsRoot(root);
        }
      } catch (error) {
        console.error("加载 workspace 目录失败:", error);
      }
    };

    void loadWorkspaceProjectsRoot();

    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    if (!createProjectDialogOpen) {
      setResolvedProjectPath("");
      setPathChecking(false);
      setPathConflictMessage("");
      return;
    }

    const projectName = newProjectName.trim();
    if (!projectName) {
      setResolvedProjectPath("");
      setPathChecking(false);
      setPathConflictMessage("");
      return;
    }

    let mounted = true;

    const resolvePath = async () => {
      try {
        const path = await resolveProjectRootPath(projectName);
        if (mounted) {
          setResolvedProjectPath(path);
        }
      } catch (error) {
        console.error("解析项目目录失败:", error);
        if (mounted) {
          setResolvedProjectPath("");
          setPathConflictMessage("");
          setPathChecking(false);
        }
      }
    };

    void resolvePath();

    return () => {
      mounted = false;
    };
  }, [createProjectDialogOpen, newProjectName]);

  useEffect(() => {
    if (!createProjectDialogOpen || !resolvedProjectPath) {
      setPathChecking(false);
      setPathConflictMessage("");
      return;
    }

    let mounted = true;
    setPathChecking(true);

    const checkPathConflict = async () => {
      try {
        const existingProject = await getProjectByRootPath(resolvedProjectPath);
        if (!mounted) {
          return;
        }

        if (existingProject) {
          setPathConflictMessage(`路径已存在项目：${existingProject.name}`);
        } else {
          setPathConflictMessage("");
        }
      } catch (error) {
        console.error("检查项目路径冲突失败:", error);
        if (mounted) {
          setPathConflictMessage("");
        }
      } finally {
        if (mounted) {
          setPathChecking(false);
        }
      }
    };

    void checkPathConflict();

    return () => {
      mounted = false;
    };
  }, [createProjectDialogOpen, resolvedProjectPath]);

  useEffect(() => {
    if (!selectedProjectId) {
      setContents([]);
      setSelectedContentId(null);
      return;
    }

    void loadContents(selectedProjectId);
  }, [loadContents, selectedProjectId, projects]);

  useEffect(() => {
    if (!selectedContentId || contentCreationModes[selectedContentId]) {
      return;
    }

    let mounted = true;

    const loadCreationMode = async () => {
      try {
        const content = await getContent(selectedContentId);
        const metadata = content?.metadata;
        const mode = parseCreationMode(
          metadata && typeof metadata === "object"
            ? (metadata as Record<string, unknown>).creationMode
            : null,
        );

        if (mounted && mode) {
          setContentCreationModes((previous) => ({
            ...previous,
            [selectedContentId]: mode,
          }));
        }
      } catch (error) {
        console.error("读取文稿创作模式失败:", error);
      }
    };

    void loadCreationMode();

    return () => {
      mounted = false;
    };
  }, [contentCreationModes, selectedContentId]);

  const handleBackHome = useCallback(() => {
    onNavigate?.("agent", buildHomeAgentParams());
  }, [onNavigate]);

  const handleBackToProjectManagement = useCallback(() => {
    setWorkspaceMode("project-management");
    setShowChatPanel(true);
    setActiveRightDrawer(null);
    setLeftSidebarCollapsed(false);
  }, [setLeftSidebarCollapsed]);

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

  // 键盘快捷键: Cmd/Ctrl + B 切换左侧栏
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "b") {
        event.preventDefault();
        toggleLeftSidebar();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [toggleLeftSidebar]);

  const shouldRenderLeftSidebar =
    workspaceMode !== "workspace" || !leftSidebarCollapsed;

  return (
    <div className="flex flex-col h-full min-h-0">
      {workspaceMode !== "workspace" && (
        <header className="h-12 border-b px-3 flex items-center gap-2 bg-background">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={handleBackHome}
            title="回到首页"
          >
            <Home className="h-4 w-4" />
          </Button>

          {workspaceMode !== "project-management" && (
            <Button
              variant="outline"
              size="sm"
              className="h-8"
              onClick={handleBackToProjectManagement}
            >
              {getProjectTypeLabel(theme)}项目管理
            </Button>
          )}

          <div className="text-sm font-medium ml-2">
            {getProjectTypeLabel(theme)}
          </div>
          {selectedProject && (
            <div className="text-xs text-muted-foreground truncate">
              {selectedProject.name}
            </div>
          )}
        </header>
      )}

      <div className="flex flex-1 min-h-0">
        {shouldRenderLeftSidebar && (
          <TooltipProvider>
            <aside
              className={cn(
                "border-r bg-muted/20 flex flex-col transition-all duration-300 ease-out",
                leftSidebarCollapsed ? "w-16" : "w-[260px] min-w-[240px]",
              )}
            >
              {leftSidebarCollapsed ? (
                // 图标模式 (折叠状态)
                <div className="flex flex-col items-center py-3 gap-4">
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-10 w-10"
                        onClick={toggleLeftSidebar}
                      >
                        <PanelLeftOpen className="h-5 w-5" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="right">
                      <p>展开侧边栏 (⌘B)</p>
                    </TooltipContent>
                  </Tooltip>

                  <div className="w-full border-t" />

                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-10 w-10"
                        onClick={toggleLeftSidebar}
                      >
                        <FolderOpen className="h-5 w-5" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="right">
                      <p>项目列表</p>
                    </TooltipContent>
                  </Tooltip>

                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-10 w-10"
                        onClick={toggleLeftSidebar}
                      >
                        <FileText className="h-5 w-5" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="right">
                      <p>文稿列表</p>
                    </TooltipContent>
                  </Tooltip>
                </div>
              ) : (
                // 完整模式 (展开状态)
                <>
                  <div className="px-3 py-3 border-b space-y-2">
                    <div className="flex items-center justify-between gap-2">
                      <div className="flex-1 min-w-0">
                        <h2 className="text-sm font-semibold truncate">
                          {getProjectTypeLabel(theme)}
                        </h2>
                        <p className="text-xs text-muted-foreground">
                          主题项目管理
                        </p>
                      </div>
                      <div className="flex items-center gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={toggleLeftSidebar}
                          title="折叠侧边栏 (⌘B)"
                        >
                          <PanelLeftClose className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => {
                            void loadProjects();
                          }}
                          disabled={projectsLoading}
                        >
                          <RefreshCw
                            className={cn(
                              "h-4 w-4",
                              projectsLoading && "animate-spin",
                            )}
                          />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={handleOpenCreateProjectDialog}
                          title="新建项目"
                        >
                          <Plus className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>

                    <Input
                      value={projectQuery}
                      onChange={(event) => setProjectQuery(event.target.value)}
                      placeholder="搜索项目..."
                      className="h-8 text-xs"
                    />
                  </div>

                  <div className="flex-1 min-h-0 flex flex-col">
                    <div className="min-h-0 basis-1/2 border-b flex flex-col">
                      <div className="px-3 py-2 text-xs text-muted-foreground">
                        项目
                      </div>
                      <ScrollArea className="flex-1">
                        <div className="p-2 space-y-1">
                          {filteredProjects.length === 0 ? (
                            <div className="px-2 py-6 text-xs text-muted-foreground text-center">
                              该主题下暂无项目
                            </div>
                          ) : (
                            filteredProjects.map((project) => (
                              <button
                                key={project.id}
                                className={cn(
                                  "w-full text-left rounded-md px-2 py-2 transition-colors",
                                  "hover:bg-accent",
                                  selectedProjectId === project.id &&
                                    "bg-accent text-accent-foreground",
                                )}
                                onClick={() => {
                                  setSelectedProjectId(project.id);
                                  setContentQuery("");
                                }}
                              >
                                <div className="flex items-center gap-2">
                                  <FolderOpen className="h-4 w-4 text-muted-foreground" />
                                  <span className="text-sm font-medium truncate">
                                    {project.name}
                                  </span>
                                </div>
                                <div className="mt-1 text-[11px] text-muted-foreground truncate">
                                  {getProjectTypeLabel(project.workspaceType)}
                                </div>
                              </button>
                            ))
                          )}
                        </div>
                      </ScrollArea>
                    </div>

                    <div className="min-h-0 basis-1/2 flex flex-col">
                      <div className="px-3 py-2 flex items-center gap-2">
                        <div className="text-xs text-muted-foreground flex-1">
                          文稿
                        </div>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7"
                          onClick={handleOpenCreateContentDialog}
                          disabled={!selectedProjectId}
                          title="新建文稿"
                        >
                          <Plus className="h-4 w-4" />
                        </Button>
                      </div>

                      <div className="px-2 pb-2">
                        <Input
                          value={contentQuery}
                          onChange={(event) =>
                            setContentQuery(event.target.value)
                          }
                          placeholder="搜索文稿..."
                          className="h-8 text-xs"
                          disabled={!selectedProjectId}
                        />
                      </div>

                      <ScrollArea className="flex-1">
                        <div className="p-2 space-y-1">
                          {contentsLoading ? (
                            <div className="px-2 py-6 text-xs text-muted-foreground text-center">
                              文稿加载中...
                            </div>
                          ) : filteredContents.length === 0 ? (
                            <div className="px-2 py-6 text-xs text-muted-foreground text-center">
                              还没有文稿
                            </div>
                          ) : (
                            filteredContents.map((content) => (
                              <button
                                key={content.id}
                                className={cn(
                                  "w-full text-left rounded-md px-2 py-2 transition-colors",
                                  "hover:bg-accent",
                                  selectedContentId === content.id &&
                                    "bg-accent text-accent-foreground",
                                )}
                                onClick={() => handleEnterWorkspace(content.id)}
                              >
                                <div className="flex items-center gap-2">
                                  <FileText className="h-4 w-4 text-muted-foreground" />
                                  <span className="text-sm font-medium truncate">
                                    {content.title}
                                  </span>
                                </div>
                                <div className="mt-1 text-[11px] text-muted-foreground truncate">
                                  {formatRelativeTime(content.updated_at)}
                                </div>
                              </button>
                            ))
                          )}
                        </div>
                      </ScrollArea>
                    </div>
                  </div>
                </>
              )}
            </aside>
          </TooltipProvider>
        )}

        <main className="flex-1 min-w-0 min-h-0 flex flex-col">
          {workspaceMode === "project-management" ? (
            <div className="h-full rounded-lg border bg-card flex flex-col items-center justify-center gap-3 text-muted-foreground m-4">
              <Sparkles className="h-8 w-8 opacity-60" />
              <p className="text-sm">先完成项目管理，再进入三栏作业界面</p>
              <p className="text-xs text-muted-foreground">
                在左侧选择文稿，或先新建文稿
              </p>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  onClick={handleOpenCreateProjectDialog}
                >
                  <FolderOpen className="h-4 w-4 mr-1" />
                  新建项目
                </Button>
                <Button
                  variant="outline"
                  onClick={handleOpenCreateContentDialog}
                  disabled={!selectedProjectId}
                >
                  <Plus className="h-4 w-4 mr-1" />
                  新建文稿并进入作业
                </Button>
                <Button
                  variant="outline"
                  onClick={handleOpenProjectDetail}
                  disabled={!selectedProjectId}
                >
                  <FolderOpen className="h-4 w-4 mr-1" />
                  项目详情
                </Button>
              </div>
            </div>
          ) : workspaceMode === "project-detail" ? (
            !selectedProjectId ? (
              <div className="h-full rounded-lg border bg-card flex flex-col items-center justify-center gap-3 text-muted-foreground m-4">
                <Sparkles className="h-8 w-8 opacity-60" />
                <p className="text-sm">请先在左侧选择项目</p>
              </div>
            ) : (
              <ProjectDetailPage
                projectId={selectedProjectId}
                onBack={handleBackToProjectManagement}
                onNavigateToChat={() => {
                  setWorkspaceMode("workspace");
                  setShowChatPanel(false);
                  setActiveRightDrawer(null);
                  setLeftSidebarCollapsed(true);
                }}
              />
            )
          ) : !selectedProjectId || !selectedContentId ? (
            <div className="h-full rounded-lg border bg-card flex flex-col items-center justify-center gap-3 text-muted-foreground m-4">
              <Sparkles className="h-8 w-8 opacity-60" />
              <p className="text-sm">请先在左侧选择项目并打开文稿</p>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  onClick={handleOpenCreateProjectDialog}
                >
                  <FolderOpen className="h-4 w-4 mr-1" />
                  新建项目
                </Button>
                <Button
                  variant="outline"
                  onClick={handleOpenCreateContentDialog}
                  disabled={!selectedProjectId}
                >
                  <Plus className="h-4 w-4 mr-1" />
                  新建文稿
                </Button>
              </div>
            </div>
          ) : (
            <div className="flex-1 min-h-0">
              <AgentChatPage
                key={`${selectedProjectId || ""}:${selectedContentId || ""}:${theme || ""}:workspace`}
                onNavigate={onNavigate}
                projectId={selectedProjectId}
                contentId={selectedContentId}
                theme={theme}
                initialUserPrompt={
                  selectedContentId
                    ? pendingInitialPromptsByContentId[selectedContentId]
                    : undefined
                }
                onInitialUserPromptConsumed={() => {
                  if (!selectedContentId) {
                    return;
                  }
                  consumePendingInitialPrompt(selectedContentId);
                }}
                initialCreationMode={
                  (selectedContentId &&
                    contentCreationModes[selectedContentId]) ||
                  undefined
                }
                lockTheme={true}
                hideHistoryToggle={true}
                showChatPanel={showChatPanel}
                onBackToProjectManagement={handleBackToProjectManagement}
                hideInlineStepProgress={true}
                onWorkflowProgressChange={setWorkflowProgress}
              />
            </div>
          )}
        </main>

        {workspaceMode === "workspace" && activeRightDrawer === "tools" && (
          <aside className="w-[260px] min-w-[260px] border-l bg-muted/10 p-4 flex flex-col gap-3">
            <h3 className="text-sm font-semibold">主题工具</h3>
            <Button
              variant="outline"
              className="justify-start"
              onClick={() => {
                void handleQuickSaveCurrent();
              }}
              disabled={!selectedContentId}
            >
              <FileText className="h-4 w-4 mr-2" />
              快速保存
            </Button>
            <Button
              variant="outline"
              className="justify-start"
              onClick={handleOpenProjectDetail}
              disabled={!selectedProjectId}
            >
              <FolderOpen className="h-4 w-4 mr-2" />
              项目详情
            </Button>
          </aside>
        )}

        {workspaceMode === "workspace" && (
          <aside className="w-14 min-w-14 border-l bg-background/95 flex flex-col items-center py-3 gap-2">
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className={cn(
                      "h-9 w-9",
                      showChatPanel && "bg-accent text-accent-foreground",
                    )}
                    onClick={() => setShowChatPanel((visible) => !visible)}
                    title={showChatPanel ? "隐藏 AI 对话" : "显示 AI 对话"}
                  >
                    <Bot className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="left">
                  <p>{showChatPanel ? "隐藏 AI 对话" : "显示 AI 对话"}</p>
                </TooltipContent>
              </Tooltip>

              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className={cn(
                      "h-9 w-9",
                      activeRightDrawer === "tools" &&
                        "bg-accent text-accent-foreground",
                    )}
                    onClick={() =>
                      setActiveRightDrawer((previous) =>
                        previous === "tools" ? null : "tools",
                      )
                    }
                    title={
                      activeRightDrawer === "tools"
                        ? "收起主题工具"
                        : "展开主题工具"
                    }
                  >
                    <Wrench className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="left">
                  <p>
                    {activeRightDrawer === "tools"
                      ? "收起主题工具"
                      : "展开主题工具"}
                  </p>
                </TooltipContent>
              </Tooltip>

              {workflowProgress && workflowProgress.steps.length > 0 && (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className={cn(
                        "h-9 w-9",
                        showWorkflowRail && "bg-accent text-accent-foreground",
                      )}
                      onClick={() =>
                        setShowWorkflowRail((previous) => !previous)
                      }
                      title={showWorkflowRail ? "收起流程步骤" : "展开流程步骤"}
                    >
                      <Sparkles className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="left">
                    <p>{showWorkflowRail ? "收起流程步骤" : "展开流程步骤"}</p>
                  </TooltipContent>
                </Tooltip>
              )}

              {workflowProgress &&
                workflowProgress.steps.length > 0 &&
                showWorkflowRail && (
                  <div className="overflow-hidden max-h-80 opacity-100 pointer-events-auto transition-all duration-200">
                    <div className="w-8 border-t my-1" />
                    <div className="flex flex-col items-center gap-1">
                      {workflowProgress.steps.map((step, index) => {
                        const isCurrent =
                          index === workflowProgress.currentIndex;
                        const isCompleted =
                          step.status === "completed" ||
                          step.status === "skipped";
                        return (
                          <Tooltip key={step.id}>
                            <TooltipTrigger asChild>
                              <button
                                type="button"
                                className={cn(
                                  "h-7 w-7 rounded-full border text-[11px] font-medium transition-colors",
                                  isCurrent &&
                                    "border-primary bg-primary/10 text-primary",
                                  !isCurrent &&
                                    isCompleted &&
                                    "border-primary/40 bg-primary/5 text-primary",
                                  !isCurrent &&
                                    !isCompleted &&
                                    "border-border bg-muted/40 text-muted-foreground",
                                )}
                              >
                                {isCompleted ? "✓" : index + 1}
                              </button>
                            </TooltipTrigger>
                            <TooltipContent side="left">
                              <p>{step.title}</p>
                              <p className="text-[11px] text-muted-foreground">
                                {isCurrent
                                  ? "当前步骤"
                                  : getWorkflowStepStatusLabel(step.status)}
                              </p>
                            </TooltipContent>
                          </Tooltip>
                        );
                      })}
                    </div>
                  </div>
                )}
            </TooltipProvider>
          </aside>
        )}
      </div>

      <Dialog
        open={createProjectDialogOpen}
        onOpenChange={(open) => {
          if (!creatingProject) {
            setCreateProjectDialogOpen(open);
          }
        }}
      >
        <DialogContent className="sm:max-w-[560px]">
          <DialogHeader>
            <DialogTitle>新建项目</DialogTitle>
            <DialogDescription>
              请输入项目名称，项目将创建到固定 workspace 目录。
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-2">
            <div className="grid gap-2">
              <Label htmlFor="workspace-project-name">项目名称</Label>
              <Input
                id="workspace-project-name"
                value={newProjectName}
                onChange={(event) => setNewProjectName(event.target.value)}
                placeholder="请输入项目名称"
                autoFocus
                disabled={creatingProject}
              />
            </div>

            <div className="grid gap-2">
              <Label htmlFor="workspace-project-type">项目类型</Label>
              <Input
                id="workspace-project-type"
                value={getProjectTypeLabel(theme as ProjectType)}
                disabled
              />
            </div>

            <div className="grid gap-2">
              <Label htmlFor="workspace-project-path">workspace 目录</Label>
              <Input
                id="workspace-project-path"
                value={workspaceProjectsRoot}
                placeholder="加载中..."
                readOnly
              />
              <p className="text-xs text-muted-foreground break-all">
                将创建到：
                {resolvedProjectPath
                  ? resolvedProjectPath
                  : newProjectName.trim()
                    ? `${workspaceProjectsRoot || "..."}/${newProjectName.trim()}`
                    : "请输入项目名称"}
              </p>
              {pathChecking && (
                <p className="text-xs text-muted-foreground">正在检查路径...</p>
              )}
              {!pathChecking && pathConflictMessage && (
                <p className="text-xs text-destructive">
                  {pathConflictMessage}
                </p>
              )}
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setCreateProjectDialogOpen(false)}
              disabled={creatingProject}
            >
              取消
            </Button>
            <Button
              onClick={() => {
                void handleCreateProject();
              }}
              disabled={
                creatingProject ||
                pathChecking ||
                !!pathConflictMessage ||
                !newProjectName.trim() ||
                !workspaceProjectsRoot.trim()
              }
            >
              {creatingProject ? "创建中..." : "创建项目"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={createContentDialogOpen}
        onOpenChange={(open) => {
          if (!creatingContent) {
            setCreateContentDialogOpen(open);
            if (!open) {
              resetCreateContentDialogState();
            }
          }
        }}
      >
        <DialogContent className="sm:max-w-[560px]">
          <DialogHeader>
            <DialogTitle>新建文稿</DialogTitle>
            <DialogDescription>
              {createContentDialogStep === "mode"
                ? "先选择创作模式，再填写创作意图。"
                : "填写创作意图后将进入 AI 对话，并按所选模式自动开始写稿。"}
            </DialogDescription>
          </DialogHeader>

          <div className="py-2 space-y-3">
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <span>
                步骤 {createContentDialogStep === "mode" ? "1/2" : "2/2"}
              </span>
              <span>
                {createContentDialogStep === "mode"
                  ? "选择创作模式"
                  : "填写创作意图"}
              </span>
            </div>

            {createContentDialogStep === "mode" ? (
              <div className="grid gap-2">
                {CREATION_MODE_OPTIONS.map((modeOption) => (
                  <Button
                    key={modeOption.value}
                    type="button"
                    variant={
                      selectedCreationMode === modeOption.value
                        ? "default"
                        : "outline"
                    }
                    className="h-auto justify-start py-3"
                    onClick={() => setSelectedCreationMode(modeOption.value)}
                    disabled={creatingContent}
                  >
                    <div className="text-left">
                      <div className="text-sm font-medium">
                        {modeOption.label}
                      </div>
                      <div
                        className={cn(
                          "text-xs mt-1",
                          selectedCreationMode === modeOption.value
                            ? "text-primary-foreground/80"
                            : "text-muted-foreground",
                        )}
                      >
                        {modeOption.description}
                      </div>
                    </div>
                  </Button>
                ))}
              </div>
            ) : (
              <div className="space-y-3">
                {currentCreationIntentFields.map((field) => (
                  <div key={field.key} className="grid gap-2">
                    <Label>{field.label}</Label>
                    {field.options && field.options.length > 0 ? (
                      <Select
                        value={creationIntentValues[field.key] || undefined}
                        onValueChange={(value) =>
                          handleCreationIntentValueChange(field.key, value)
                        }
                        disabled={creatingContent}
                      >
                        <SelectTrigger>
                          <SelectValue placeholder={field.placeholder} />
                        </SelectTrigger>
                        <SelectContent>
                          {field.options.map((option) => (
                            <SelectItem key={option.value} value={option.value}>
                              {option.label}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    ) : field.multiline ? (
                      <Textarea
                        id={`creation-intent-${field.key}`}
                        value={creationIntentValues[field.key]}
                        onChange={(event) =>
                          handleCreationIntentValueChange(
                            field.key,
                            event.target.value,
                          )
                        }
                        placeholder={field.placeholder}
                        className="min-h-[84px] resize-y"
                        disabled={creatingContent}
                      />
                    ) : (
                      <Input
                        id={`creation-intent-${field.key}`}
                        value={creationIntentValues[field.key]}
                        onChange={(event) =>
                          handleCreationIntentValueChange(
                            field.key,
                            event.target.value,
                          )
                        }
                        placeholder={field.placeholder}
                        disabled={creatingContent}
                      />
                    )}
                  </div>
                ))}

                <div className="grid gap-2">
                  <Label htmlFor="creation-intent-extra">补充要求</Label>
                  <Textarea
                    id="creation-intent-extra"
                    value={creationIntentValues.extraRequirements}
                    onChange={(event) =>
                      handleCreationIntentValueChange(
                        "extraRequirements",
                        event.target.value,
                      )
                    }
                    placeholder="可补充风格、禁忌词、信息来源、输出格式等"
                    className="min-h-[96px] resize-y"
                    disabled={creatingContent}
                  />
                </div>

                <div className="space-y-1">
                  <p
                    className={cn(
                      "text-xs",
                      currentIntentLength < MIN_CREATION_INTENT_LENGTH
                        ? "text-destructive"
                        : "text-muted-foreground",
                    )}
                  >
                    创作意图字数：{currentIntentLength}/
                    {MIN_CREATION_INTENT_LENGTH}
                  </p>
                  {creationIntentError && (
                    <p className="text-xs text-destructive">
                      {creationIntentError}
                    </p>
                  )}
                </div>
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                if (createContentDialogStep === "intent") {
                  setCreateContentDialogStep("mode");
                  setCreationIntentError("");
                  return;
                }
                setCreateContentDialogOpen(false);
                resetCreateContentDialogState();
              }}
              disabled={creatingContent}
            >
              {createContentDialogStep === "mode" ? "取消" : "上一步"}
            </Button>
            <Button
              onClick={() => {
                if (createContentDialogStep === "mode") {
                  handleGoToIntentStep();
                  return;
                }
                void handleCreateContent();
              }}
              disabled={
                !selectedProjectId ||
                creatingContent ||
                (createContentDialogStep === "intent" &&
                  currentIntentLength < MIN_CREATION_INTENT_LENGTH)
              }
            >
              {createContentDialogStep === "mode"
                ? "下一步"
                : creatingContent
                  ? "创建中..."
                  : "创建并进入作业"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default WorkbenchPage;
