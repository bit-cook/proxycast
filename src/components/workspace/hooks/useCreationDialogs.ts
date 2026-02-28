import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { toast } from "sonner";
import {
  createContent,
  createProject,
  extractErrorMessage,
  getContent,
  getContentTypeLabel,
  getCreateProjectErrorMessage,
  getDefaultContentTypeForProject,
  getProjectByRootPath,
  getProjectTypeLabel,
  getWorkspaceProjectsRoot,
  resolveProjectRootPath,
  type ProjectType,
} from "@/lib/api/project";
import type { WorkspaceTheme } from "@/types/page";
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

type CreateContentDialogStep = "mode" | "intent";

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

function parseCreationModeFromMetadata(metadata: unknown): CreationMode | null {
  if (!metadata || typeof metadata !== "object") {
    return null;
  }
  return parseCreationMode((metadata as Record<string, unknown>).creationMode);
}

function useWorkspaceProjectsRootLoader(
  setWorkspaceProjectsRoot: Dispatch<SetStateAction<string>>,
): void {
  useEffect(() => {
    let mounted = true;

    const loadWorkspaceRoot = async () => {
      try {
        const root = await getWorkspaceProjectsRoot();
        if (mounted) {
          setWorkspaceProjectsRoot(root);
        }
      } catch (error) {
        console.error("加载 workspace 目录失败:", error);
      }
    };

    void loadWorkspaceRoot();

    return () => {
      mounted = false;
    };
  }, [setWorkspaceProjectsRoot]);
}

interface UseProjectPathResolverParams {
  createProjectDialogOpen: boolean;
  newProjectName: string;
  resetProjectPathState: () => void;
  setResolvedProjectPath: Dispatch<SetStateAction<string>>;
}

function useProjectPathResolver({
  createProjectDialogOpen,
  newProjectName,
  resetProjectPathState,
  setResolvedProjectPath,
}: UseProjectPathResolverParams): void {
  useEffect(() => {
    if (!createProjectDialogOpen) {
      resetProjectPathState();
      return;
    }

    const projectName = newProjectName.trim();
    if (!projectName) {
      resetProjectPathState();
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
          resetProjectPathState();
        }
      }
    };

    void resolvePath();

    return () => {
      mounted = false;
    };
  }, [
    createProjectDialogOpen,
    newProjectName,
    resetProjectPathState,
    setResolvedProjectPath,
  ]);
}

interface UseProjectPathConflictCheckerParams {
  createProjectDialogOpen: boolean;
  resolvedProjectPath: string;
  setPathChecking: Dispatch<SetStateAction<boolean>>;
  setPathConflictMessage: Dispatch<SetStateAction<string>>;
}

function useProjectPathConflictChecker({
  createProjectDialogOpen,
  resolvedProjectPath,
  setPathChecking,
  setPathConflictMessage,
}: UseProjectPathConflictCheckerParams): void {
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
  }, [
    createProjectDialogOpen,
    resolvedProjectPath,
    setPathChecking,
    setPathConflictMessage,
  ]);
}

interface UseContentCreationModeLoaderParams {
  selectedContentId: string | null;
  contentCreationModes: Record<string, CreationMode>;
  setContentCreationModes: Dispatch<SetStateAction<Record<string, CreationMode>>>;
}

function useContentCreationModeLoader({
  selectedContentId,
  contentCreationModes,
  setContentCreationModes,
}: UseContentCreationModeLoaderParams): void {
  useEffect(() => {
    if (!selectedContentId || contentCreationModes[selectedContentId]) {
      return;
    }

    let mounted = true;
    const loadCreationMode = async () => {
      try {
        const content = await getContent(selectedContentId);
        const mode = parseCreationModeFromMetadata(content?.metadata);

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
  }, [contentCreationModes, selectedContentId, setContentCreationModes]);
}

export interface UseCreationDialogsParams {
  theme: WorkspaceTheme;
  selectedProjectId: string | null;
  selectedContentId: string | null;
  loadProjects: () => Promise<void>;
  loadContents: (projectId: string) => Promise<void>;
  onEnterWorkspace: (
    contentId: string,
    options?: {
      showChatPanel?: boolean;
    },
  ) => void;
  onProjectCreated: (projectId: string) => void;
  defaultCreationMode: CreationMode;
  minCreationIntentLength: number;
}

export function useCreationDialogs({
  theme,
  selectedProjectId,
  selectedContentId,
  loadProjects,
  loadContents,
  onEnterWorkspace,
  onProjectCreated,
  defaultCreationMode,
  minCreationIntentLength,
}: UseCreationDialogsParams) {
  const [createProjectDialogOpen, setCreateProjectDialogOpen] = useState(false);
  const [createContentDialogOpen, setCreateContentDialogOpen] = useState(false);
  const [createContentDialogStep, setCreateContentDialogStep] =
    useState<CreateContentDialogStep>("mode");
  const [newProjectName, setNewProjectName] = useState("");
  const [workspaceProjectsRoot, setWorkspaceProjectsRoot] = useState("");
  const [creatingProject, setCreatingProject] = useState(false);
  const [creatingContent, setCreatingContent] = useState(false);
  const [selectedCreationMode, setSelectedCreationMode] =
    useState<CreationMode>(defaultCreationMode);
  const [creationIntentValues, setCreationIntentValues] =
    useState<CreationIntentFormValues>(() => createInitialCreationIntentValues());
  const [creationIntentError, setCreationIntentError] = useState("");
  const [resolvedProjectPath, setResolvedProjectPath] = useState("");
  const [pathChecking, setPathChecking] = useState(false);
  const [pathConflictMessage, setPathConflictMessage] = useState("");
  const [pendingInitialPromptsByContentId, setPendingInitialPromptsByContentId] =
    useState<Record<string, string>>({});
  const [contentCreationModes, setContentCreationModes] = useState<
    Record<string, CreationMode>
  >({});

  const resetProjectPathState = useCallback(() => {
    setResolvedProjectPath("");
    setPathChecking(false);
    setPathConflictMessage("");
  }, []);

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
    () =>
      validateCreationIntent(creationIntentInput, minCreationIntentLength).length,
    [creationIntentInput, minCreationIntentLength],
  );

  const resetCreateContentDialogState = useCallback(() => {
    setCreateContentDialogStep("mode");
    setSelectedCreationMode(defaultCreationMode);
    setCreationIntentValues(createInitialCreationIntentValues());
    setCreationIntentError("");
  }, [defaultCreationMode]);

  const handleOpenCreateProjectDialog = useCallback(() => {
    setNewProjectName(`${getProjectTypeLabel(theme as ProjectType)}项目`);
    resetProjectPathState();
    setCreateProjectDialogOpen(true);
  }, [resetProjectPathState, theme]);

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
      onProjectCreated(createdProject.id);
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
  }, [loadProjects, newProjectName, onProjectCreated, theme]);

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

  const handleCreateContent = useCallback(async () => {
    if (!selectedProjectId) {
      return;
    }

    const validation = validateCreationIntent(
      creationIntentInput,
      minCreationIntentLength,
    );
    if (!validation.valid) {
      setCreationIntentError(validation.message || "请完善创作意图");
      return;
    }

    const initialUserPrompt = buildCreationIntentPrompt(creationIntentInput);
    const creationIntentMetadata =
      buildCreationIntentMetadata(creationIntentInput);

    setCreatingContent(true);
    try {
      const defaultType = getDefaultContentTypeForProject(theme as ProjectType);
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
      onEnterWorkspace(created.id, { showChatPanel: true });
      toast.success("已创建新文稿");
    } catch (error) {
      console.error("创建文稿失败:", error);
      toast.error("创建文稿失败");
    } finally {
      setCreatingContent(false);
    }
  }, [
    creationIntentInput,
    loadContents,
    minCreationIntentLength,
    onEnterWorkspace,
    resetCreateContentDialogState,
    selectedCreationMode,
    selectedProjectId,
    theme,
  ]);

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

  useWorkspaceProjectsRootLoader(setWorkspaceProjectsRoot);
  useProjectPathResolver({
    createProjectDialogOpen,
    newProjectName,
    resetProjectPathState,
    setResolvedProjectPath,
  });
  useProjectPathConflictChecker({
    createProjectDialogOpen,
    resolvedProjectPath,
    setPathChecking,
    setPathConflictMessage,
  });
  useContentCreationModeLoader({
    selectedContentId,
    contentCreationModes,
    setContentCreationModes,
  });

  return {
    createProjectDialogOpen,
    setCreateProjectDialogOpen,
    createContentDialogOpen,
    setCreateContentDialogOpen,
    createContentDialogStep,
    setCreateContentDialogStep,
    newProjectName,
    setNewProjectName,
    workspaceProjectsRoot,
    creatingProject,
    creatingContent,
    selectedCreationMode,
    setSelectedCreationMode,
    creationIntentValues,
    creationIntentError,
    setCreationIntentError,
    currentCreationIntentFields,
    currentIntentLength,
    pendingInitialPromptsByContentId,
    contentCreationModes,
    resolvedProjectPath,
    pathChecking,
    pathConflictMessage,
    resetCreateContentDialogState,
    handleOpenCreateProjectDialog,
    handleCreateProject,
    handleOpenCreateContentDialog,
    handleCreationIntentValueChange,
    handleGoToIntentStep,
    handleCreateContent,
    consumePendingInitialPrompt,
  };
}

export default useCreationDialogs;
