import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import {
  checkNovelConsistency,
  createNovelProject,
  generateNovelCharacters,
  generateNovelOutline,
  getNovelProjectSnapshot,
  listNovelRuns,
  polishNovelChapter,
  rewriteNovelChapter,
  updateNovelSettings,
  type NovelGenerationRun,
  type NovelProjectSnapshot,
} from "@/lib/api/novel";
import NovelSettingsPanel from "@/components/projects/tabs/NovelSettingsPanel";
import NovelSettingsWizard from "@/components/projects/tabs/novel-settings/NovelSettingsWizard";
import {
  createDefaultNovelSettingsEnvelope,
  normalizeNovelSettingsEnvelope,
  type NovelSettingsEnvelope,
} from "@/lib/novel-settings/types";
import {
  resolveNovelPipelineState,
  type NovelPipelineStage,
  type NovelStageStatus,
} from "@/lib/novel-flow/pipeline";
import { generateNextChapterWithConsistency } from "@/lib/novel-flow/actions";
import {
  AlertTriangle,
  BookOpenText,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Loader2,
  RefreshCw,
  Sparkles,
  Users,
} from "lucide-react";
import { toast } from "sonner";

interface StageMeta {
  id: NovelPipelineStage;
  title: string;
  description: string;
}

const SETTINGS_AUTOSAVE_DELAY_MS = 800;

const STAGE_METAS: StageMeta[] = [
  {
    id: "setup",
    title: "设定",
    description: "完成小说设定与约束",
  },
  {
    id: "outline",
    title: "大纲",
    description: "生成并校验故事骨架",
  },
  {
    id: "characters",
    title: "角色",
    description: "生成角色阵列",
  },
  {
    id: "chapters",
    title: "章节",
    description: "半自动生成与迭代",
  },
  {
    id: "qa",
    title: "质检",
    description: "一致性与质量检查",
  },
  {
    id: "publish",
    title: "发布",
    description: "确认发布前条件",
  },
];

type SettingsMode = "wizard" | "expert";
type FlowAction =
  | "outline"
  | "characters"
  | "chapter-rewrite"
  | "chapter-polish"
  | "chapter-consistency"
  | "chapter-cycle"
  | "save-settings"
  | "primary"
  | "refresh";

export interface NovelFlowWorkbenchProps {
  projectId: string;
  projectName?: string;
}

function getSettingsModeStorageKey(projectId: string): string {
  return `novel_settings_mode_${projectId}`;
}

function loadSettingsMode(projectId: string): SettingsMode {
  if (typeof window === "undefined") {
    return "wizard";
  }
  const raw = window.localStorage.getItem(getSettingsModeStorageKey(projectId));
  if (raw === "expert") {
    return "expert";
  }
  return "wizard";
}

function persistSettingsMode(projectId: string, mode: SettingsMode): void {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.setItem(getSettingsModeStorageKey(projectId), mode);
}

function loadPreferredModel(projectId: string): string | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }
  const keys = [
    `agent_pref_model_${projectId}`,
    "agent_pref_model_global",
    "agent_pref_model",
  ];

  for (const key of keys) {
    const raw = window.localStorage.getItem(key);
    if (!raw) {
      continue;
    }
    try {
      const parsed = JSON.parse(raw) as unknown;
      if (typeof parsed === "string" && parsed.trim()) {
        return parsed.trim();
      }
    } catch {
      if (raw.trim()) {
        return raw.trim();
      }
    }
  }

  return undefined;
}

function loadPreferredProvider(projectId: string): string | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }
  const keys = [
    `agent_pref_provider_${projectId}`,
    "agent_pref_provider_global",
    "agent_pref_provider",
  ];

  for (const key of keys) {
    const raw = window.localStorage.getItem(key);
    if (!raw) {
      continue;
    }
    try {
      const parsed = JSON.parse(raw) as unknown;
      if (typeof parsed === "string" && parsed.trim()) {
        return parsed.trim();
      }
    } catch {
      if (raw.trim()) {
        return raw.trim();
      }
    }
  }

  return undefined;
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "未知错误";
}

function isNovelProjectNotFound(message: string): boolean {
  return message.includes("小说项目不存在") || message.includes("项目不存在");
}

function formatDateTime(timestamp: number): string {
  return new Date(timestamp).toLocaleString("zh-CN", { hour12: false });
}

function formatRunMode(mode: string): string {
  switch (mode) {
    case "outline":
      return "生成大纲";
    case "characters":
      return "生成角色";
    case "generate":
      return "生成章节";
    case "continue":
      return "续写章节";
    case "rewrite":
      return "重写章节";
    case "polish":
      return "润色章节";
    default:
      return mode;
  }
}

function formatRunStatus(status: string): string {
  if (status === "success") {
    return "成功";
  }
  if (status === "failed") {
    return "失败";
  }
  return status;
}

function getRunStatusVariant(
  status: string,
): "default" | "secondary" | "destructive" | "outline" {
  if (status === "success") {
    return "default";
  }
  if (status === "failed") {
    return "destructive";
  }
  return "outline";
}

function getStageStatusVariant(
  status: NovelStageStatus,
): "default" | "secondary" | "destructive" | "outline" {
  if (status === "done") {
    return "default";
  }
  if (status === "warning") {
    return "destructive";
  }
  if (status === "ready") {
    return "secondary";
  }
  return "outline";
}

function getStageStatusLabel(status: NovelStageStatus): string {
  if (status === "done") {
    return "已完成";
  }
  if (status === "warning") {
    return "有告警";
  }
  if (status === "ready") {
    return "进行中";
  }
  return "待开始";
}

export function NovelFlowWorkbench({
  projectId,
  projectName,
}: NovelFlowWorkbenchProps) {
  const [snapshot, setSnapshot] = useState<NovelProjectSnapshot | null>(null);
  const [runs, setRuns] = useState<NovelGenerationRun[]>([]);
  const [loadingSnapshot, setLoadingSnapshot] = useState(false);
  const [loadingRuns, setLoadingRuns] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [initializing, setInitializing] = useState(false);
  const [runningAction, setRunningAction] = useState<FlowAction | null>(null);
  const [selectedChapterId, setSelectedChapterId] = useState("");
  const [rewriteInstructions, setRewriteInstructions] = useState("");
  const [polishFocus, setPolishFocus] = useState("");

  const [settingsDraft, setSettingsDraft] = useState<NovelSettingsEnvelope>(
    createDefaultNovelSettingsEnvelope(),
  );
  const [settingsMode, setSettingsMode] = useState<SettingsMode>(() =>
    loadSettingsMode(projectId),
  );
  const [settingsDirty, setSettingsDirty] = useState(false);
  const [savingSettings, setSavingSettings] = useState(false);
  const [settingsSaveError, setSettingsSaveError] = useState<string | null>(null);
  const [lastSavedAt, setLastSavedAt] = useState<number | null>(null);
  const [activeStage, setActiveStage] = useState<NovelPipelineStage>("setup");

  const autoSaveTimerRef = useRef<number | null>(null);

  const preferredModel = loadPreferredModel(projectId);
  const preferredProvider = loadPreferredProvider(projectId);

  const selectedChapter = useMemo(
    () => snapshot?.chapters.find((chapter) => chapter.id === selectedChapterId),
    [selectedChapterId, snapshot],
  );

  const pipelineState = useMemo(
    () =>
      resolveNovelPipelineState({
        snapshot,
        settings: settingsDraft,
      }),
    [settingsDraft, snapshot],
  );

  const hasSnapshot = snapshot !== null;
  const generationLocked = runningAction !== null || savingSettings;

  const settingsSyncState = useMemo<"saving" | "error" | "dirty" | "synced">(() => {
    if (savingSettings) {
      return "saving";
    }
    if (settingsSaveError) {
      return "error";
    }
    if (settingsDirty) {
      return "dirty";
    }
    return "synced";
  }, [savingSettings, settingsDirty, settingsSaveError]);

  const clearAutoSaveTimer = useCallback(() => {
    if (autoSaveTimerRef.current !== null) {
      window.clearTimeout(autoSaveTimerRef.current);
      autoSaveTimerRef.current = null;
    }
  }, []);

  const runAction = useCallback(
    async (action: FlowAction, successText: string, fn: () => Promise<void>) => {
      setRunningAction(action);
      try {
        await fn();
        toast.success(successText);
      } catch (error) {
        toast.error(`${successText}失败: ${getErrorMessage(error)}`);
      } finally {
        setRunningAction(null);
      }
    },
    [],
  );

  const loadSnapshot = useCallback(async () => {
    setLoadingSnapshot(true);
    try {
      const result = await getNovelProjectSnapshot(projectId);
      setSnapshot(result);
      setLoadError(null);
    } catch (error) {
      const message = getErrorMessage(error);
      if (isNovelProjectNotFound(message)) {
        setSnapshot(null);
        setLoadError(null);
      } else {
        setLoadError(message);
      }
    } finally {
      setLoadingSnapshot(false);
    }
  }, [projectId]);

  const loadRuns = useCallback(async () => {
    setLoadingRuns(true);
    try {
      const result = await listNovelRuns({
        project_id: projectId,
        limit: 20,
      });
      setRuns(result);
    } catch (error) {
      const message = getErrorMessage(error);
      if (isNovelProjectNotFound(message)) {
        setRuns([]);
      } else {
        toast.error(`加载运行记录失败: ${message}`);
      }
    } finally {
      setLoadingRuns(false);
    }
  }, [projectId]);

  const refreshNovelData = useCallback(async () => {
    await Promise.all([loadSnapshot(), loadRuns()]);
  }, [loadRuns, loadSnapshot]);

  useEffect(() => {
    void refreshNovelData();
  }, [refreshNovelData]);

  useEffect(() => {
    setSettingsMode(loadSettingsMode(projectId));
  }, [projectId]);

  useEffect(() => () => clearAutoSaveTimer(), [clearAutoSaveTimer]);

  useEffect(() => {
    if (!snapshot) {
      setSelectedChapterId("");
      return;
    }
    const hasSelected = snapshot.chapters.some(
      (chapter) => chapter.id === selectedChapterId,
    );
    if (hasSelected) {
      return;
    }
    const latestChapter = snapshot.chapters[snapshot.chapters.length - 1];
    setSelectedChapterId(latestChapter?.id ?? "");
  }, [selectedChapterId, snapshot]);

  useEffect(() => {
    if (!snapshot) {
      setSettingsDraft(createDefaultNovelSettingsEnvelope());
      setSettingsDirty(false);
      setSettingsSaveError(null);
      setLastSavedAt(null);
      return;
    }

    const latestSettings = normalizeNovelSettingsEnvelope(
      snapshot.latest_settings?.settings_json,
    );
    setSettingsDraft(latestSettings);
    setSettingsDirty(false);
    setSettingsSaveError(null);
    setLastSavedAt(snapshot.latest_settings?.created_at ?? null);
  }, [snapshot]);

  const handleInitializeNovelProject = useCallback(async () => {
    setInitializing(true);
    try {
      await createNovelProject({
        id: projectId,
        title: projectName?.trim() || "小说项目",
        theme: "长篇小说",
        metadata_json: {
          workspace_project_id: projectId,
        },
      });
      toast.success("小说项目初始化完成");
      await refreshNovelData();
    } catch (error) {
      toast.error(`初始化失败: ${getErrorMessage(error)}`);
    } finally {
      setInitializing(false);
    }
  }, [projectId, projectName, refreshNovelData]);

  const handleSaveSettings = useCallback(
    async (silent = false): Promise<boolean> => {
      clearAutoSaveTimer();
      if (!snapshot || savingSettings) {
        return false;
      }
      if (!settingsDirty) {
        return true;
      }

      setSavingSettings(true);
      setSettingsSaveError(null);
      try {
        const normalized = normalizeNovelSettingsEnvelope(settingsDraft);
        const saved = await updateNovelSettings({
          project_id: projectId,
          settings_json: normalized,
        });

        setSettingsDraft(normalizeNovelSettingsEnvelope(saved.settings_json));
        setSettingsDirty(false);
        setSettingsSaveError(null);
        setLastSavedAt(saved.created_at);

        if (!silent) {
          toast.success("创作设定已保存");
        }
        return true;
      } catch (error) {
        const message = getErrorMessage(error);
        setSettingsSaveError(message);
        if (!silent) {
          toast.error(`保存创作设定失败: ${message}`);
        }
        return false;
      } finally {
        setSavingSettings(false);
      }
    },
    [clearAutoSaveTimer, projectId, savingSettings, settingsDirty, settingsDraft, snapshot],
  );

  useEffect(() => {
    if (!snapshot || !settingsDirty || savingSettings) {
      return;
    }
    clearAutoSaveTimer();
    autoSaveTimerRef.current = window.setTimeout(() => {
      void handleSaveSettings(true);
    }, SETTINGS_AUTOSAVE_DELAY_MS);

    return () => {
      clearAutoSaveTimer();
    };
  }, [
    clearAutoSaveTimer,
    handleSaveSettings,
    savingSettings,
    settingsDirty,
    settingsDraft,
    snapshot,
  ]);

  const flushPendingSettingsSave = useCallback(async (): Promise<boolean> => {
    clearAutoSaveTimer();
    return handleSaveSettings(true);
  }, [clearAutoSaveTimer, handleSaveSettings]);

  const ensureSettingsSavedForGeneration = useCallback(async () => {
    const saved = await flushPendingSettingsSave();
    if (!saved) {
      throw new Error("创作设定保存失败");
    }
  }, [flushPendingSettingsSave]);

  const handleSettingsChange = useCallback((nextData: NovelSettingsEnvelope["data"]) => {
    setSettingsDraft((prev) => ({
      ...normalizeNovelSettingsEnvelope(prev),
      data: nextData,
    }));
    setSettingsDirty(true);
    setSettingsSaveError(null);
  }, []);

  const handleSwitchSettingsMode = useCallback(
    async (mode: SettingsMode) => {
      if (mode === settingsMode || runningAction !== null || savingSettings) {
        return;
      }

      const saved = await flushPendingSettingsSave();
      if (!saved) {
        toast.error("切换模式前保存失败，请稍后重试");
        return;
      }

      setSettingsMode(mode);
      persistSettingsMode(projectId, mode);
    },
    [flushPendingSettingsSave, projectId, runningAction, savingSettings, settingsMode],
  );

  const handleGenerateOutline = useCallback(async () => {
    await runAction("outline", "大纲已更新", async () => {
      await ensureSettingsSavedForGeneration();
      await generateNovelOutline({
        project_id: projectId,
        provider: preferredProvider,
        model: preferredModel,
      });
      await refreshNovelData();
    });
  }, [
    ensureSettingsSavedForGeneration,
    preferredModel,
    preferredProvider,
    projectId,
    refreshNovelData,
    runAction,
  ]);

  const handleGenerateCharacters = useCallback(async () => {
    await runAction("characters", "角色阵列已更新", async () => {
      await ensureSettingsSavedForGeneration();
      await generateNovelCharacters({
        project_id: projectId,
        provider: preferredProvider,
        model: preferredModel,
      });
      await refreshNovelData();
    });
  }, [
    ensureSettingsSavedForGeneration,
    preferredModel,
    preferredProvider,
    projectId,
    refreshNovelData,
    runAction,
  ]);

  const handleGenerateNextChapter = useCallback(async () => {
    await runAction("chapter-cycle", "章节已生成", async () => {
      await ensureSettingsSavedForGeneration();
      const result = await generateNextChapterWithConsistency({
        projectId,
        hasExistingChapters: Boolean(snapshot?.chapters.length),
        provider: preferredProvider,
        model: preferredModel,
      });

      if (result.consistency) {
        toast.success(`一致性评分: ${result.consistency.score.toFixed(1)}`);
      } else if (result.consistencyError) {
        toast.warning(`章节已生成，一致性检查失败: ${result.consistencyError}`);
      }

      await refreshNovelData();
      setSelectedChapterId(result.chapter.id);
    });
  }, [
    ensureSettingsSavedForGeneration,
    preferredModel,
    preferredProvider,
    projectId,
    refreshNovelData,
    runAction,
    snapshot?.chapters.length,
  ]);

  const handleRewriteChapter = useCallback(async () => {
    if (!selectedChapterId) {
      toast.error("请先选择章节");
      return;
    }

    await runAction("chapter-rewrite", "章节已重写", async () => {
      await ensureSettingsSavedForGeneration();
      await rewriteNovelChapter({
        project_id: projectId,
        chapter_id: selectedChapterId,
        instructions: rewriteInstructions.trim() || undefined,
        provider: preferredProvider,
        model: preferredModel,
      });
      await refreshNovelData();
    });
  }, [
    ensureSettingsSavedForGeneration,
    preferredModel,
    preferredProvider,
    projectId,
    refreshNovelData,
    rewriteInstructions,
    runAction,
    selectedChapterId,
  ]);

  const handlePolishChapter = useCallback(async () => {
    if (!selectedChapterId) {
      toast.error("请先选择章节");
      return;
    }

    await runAction("chapter-polish", "章节已润色", async () => {
      await ensureSettingsSavedForGeneration();
      await polishNovelChapter({
        project_id: projectId,
        chapter_id: selectedChapterId,
        focus: polishFocus.trim() || undefined,
        provider: preferredProvider,
        model: preferredModel,
      });
      await refreshNovelData();
    });
  }, [
    ensureSettingsSavedForGeneration,
    polishFocus,
    preferredModel,
    preferredProvider,
    projectId,
    refreshNovelData,
    runAction,
    selectedChapterId,
  ]);

  const handleCheckConsistencyForSelectedChapter = useCallback(async () => {
    if (!selectedChapterId) {
      toast.error("请先选择章节");
      return;
    }

    await runAction("chapter-consistency", "一致性检查完成", async () => {
      await ensureSettingsSavedForGeneration();
      const result = await checkNovelConsistency({
        project_id: projectId,
        chapter_id: selectedChapterId,
      });
      toast.success(`一致性评分: ${result.score.toFixed(1)}`);
      await refreshNovelData();
    });
  }, [
    ensureSettingsSavedForGeneration,
    projectId,
    refreshNovelData,
    runAction,
    selectedChapterId,
  ]);

  const handlePrimaryAction = useCallback(async () => {
    if (pipelineState.primaryAction.disabled || generationLocked) {
      return;
    }

    switch (pipelineState.primaryAction.key) {
      case "save-settings":
        await handleSaveSettings(false);
        return;
      case "generate-outline":
        await handleGenerateOutline();
        return;
      case "generate-characters":
        await handleGenerateCharacters();
        return;
      case "generate-next-chapter":
        await handleGenerateNextChapter();
        return;
      case "run-consistency":
        await handleCheckConsistencyForSelectedChapter();
        return;
      case "open-publish":
        toast.info("请切换到“发布”标签页继续发布流程");
        return;
      default:
        return;
    }
  }, [
    generationLocked,
    handleCheckConsistencyForSelectedChapter,
    handleGenerateCharacters,
    handleGenerateNextChapter,
    handleGenerateOutline,
    handleSaveSettings,
    pipelineState.primaryAction,
  ]);

  if (loadingSnapshot && !hasSnapshot) {
    return (
      <div className="p-4">
        <Card>
          <CardContent className="pt-6 flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            正在加载小说编排数据...
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!hasSnapshot) {
    return (
      <div className="p-4 space-y-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">尚未初始化小说项目</CardTitle>
            <CardDescription>
              当前工作区还没有小说编排数据，初始化后即可进入流水线生成。
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {loadError && <div className="text-sm text-destructive">加载失败: {loadError}</div>}
            <Button onClick={handleInitializeNovelProject} disabled={initializing}>
              {initializing ? (
                <Loader2 className="h-4 w-4 mr-1 animate-spin" />
              ) : (
                <Sparkles className="h-4 w-4 mr-1" />
              )}
              初始化小说项目
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="p-4 space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-medium">小说流水线工作台</h2>
          <p className="text-sm text-muted-foreground">
            设定 → 大纲 → 角色 → 章节循环 → 质检 → 发布
          </p>
          {preferredModel && (
            <p className="text-xs text-muted-foreground mt-1">
              默认模型:
              <span className="font-mono"> {preferredModel}</span>
              {preferredProvider && <span className="ml-2">Provider: {preferredProvider}</span>}
            </p>
          )}
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            onClick={() => {
              void refreshNovelData();
            }}
            disabled={loadingSnapshot || loadingRuns || generationLocked}
          >
            <RefreshCw
              className={cn(
                "h-4 w-4 mr-1",
                (loadingSnapshot || loadingRuns) && "animate-spin",
              )}
            />
            刷新
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader className="pb-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="space-y-1">
              <CardTitle className="text-base flex items-center gap-2">
                当前阶段：
                {STAGE_METAS.find((item) => item.id === pipelineState.currentStage)?.title}
              </CardTitle>
              <CardDescription>
                {STAGE_METAS.find((item) => item.id === pipelineState.currentStage)?.description}
              </CardDescription>
            </div>
            <Button
              onClick={() => {
                void handlePrimaryAction();
              }}
              disabled={pipelineState.primaryAction.disabled || generationLocked}
            >
              {generationLocked ? (
                <Loader2 className="h-4 w-4 mr-1 animate-spin" />
              ) : (
                <ChevronRight className="h-4 w-4 mr-1" />
              )}
              {pipelineState.primaryAction.label}
            </Button>
          </div>
        </CardHeader>
      </Card>

      <div className="grid gap-3 xl:grid-cols-[220px_minmax(0,1fr)] 2xl:grid-cols-[220px_minmax(0,1fr)_280px]">
        <Card className="min-w-0">
          <CardHeader>
            <CardTitle className="text-sm">阶段导航</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {STAGE_METAS.map((stage) => {
              const status = pipelineState.stageStatus[stage.id];
              const selected = activeStage === stage.id;
              return (
                <button
                  key={stage.id}
                  type="button"
                  onClick={() => {
                    setActiveStage(stage.id);
                  }}
                  className={cn(
                    "w-full rounded-md border px-2 py-2 text-left transition-colors",
                    selected ? "border-primary bg-muted/40" : "hover:bg-muted/30",
                  )}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-sm font-medium">{stage.title}</span>
                    <Badge variant={getStageStatusVariant(status)}>
                      {getStageStatusLabel(status)}
                    </Badge>
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">{stage.description}</div>
                </button>
              );
            })}
          </CardContent>
        </Card>

        <div className="space-y-3 min-w-0">
          {activeStage === "setup" && (
            <>
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">创作设定</CardTitle>
                  <CardDescription>向导模式默认开启，专家模式放在二级入口。</CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge
                      variant={
                        settingsSyncState === "error"
                          ? "destructive"
                          : settingsSyncState === "dirty"
                            ? "secondary"
                            : "outline"
                      }
                    >
                      {settingsSyncState === "saving"
                        ? "设定保存中"
                        : settingsSyncState === "error"
                          ? "设定保存失败"
                          : settingsSyncState === "dirty"
                            ? "设定未保存"
                            : "设定已同步"}
                    </Badge>

                    {savingSettings && <Loader2 className="h-3.5 w-3.5 animate-spin" />}

                    {lastSavedAt && (
                      <span className="text-xs text-muted-foreground">
                        上次保存: {formatDateTime(lastSavedAt)}
                      </span>
                    )}
                    {snapshot.latest_settings && (
                      <span className="text-xs text-muted-foreground">
                        版本 {snapshot.latest_settings.version}
                      </span>
                    )}
                  </div>

                  {settingsSaveError && (
                    <div className="text-xs text-destructive">{settingsSaveError}</div>
                  )}

                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      size="sm"
                      variant={settingsMode === "wizard" ? "default" : "outline"}
                      onClick={() => {
                        void handleSwitchSettingsMode("wizard");
                      }}
                      disabled={savingSettings || generationLocked}
                    >
                      向导模式
                    </Button>
                    <Button
                      size="sm"
                      variant={settingsMode === "expert" ? "default" : "outline"}
                      onClick={() => {
                        void handleSwitchSettingsMode("expert");
                      }}
                      disabled={savingSettings || generationLocked}
                    >
                      专家模式
                    </Button>
                    <Button
                      variant={settingsDirty ? "default" : "outline"}
                      onClick={() => {
                        void handleSaveSettings(false);
                      }}
                      disabled={!settingsDirty || savingSettings || generationLocked}
                    >
                      保存设定
                    </Button>
                  </div>
                </CardContent>
              </Card>

              {settingsMode === "wizard" ? (
                <NovelSettingsWizard
                  value={settingsDraft.data}
                  onChange={handleSettingsChange}
                  onBeforeStepChange={flushPendingSettingsSave}
                  disabled={savingSettings || generationLocked}
                />
              ) : (
                <NovelSettingsPanel
                  value={settingsDraft.data}
                  onChange={handleSettingsChange}
                  disabled={savingSettings || generationLocked}
                />
              )}
            </>
          )}

          {activeStage === "outline" && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">大纲阶段</CardTitle>
                <CardDescription>
                  {snapshot.latest_outline
                    ? `版本 ${snapshot.latest_outline.version} · ${formatDateTime(snapshot.latest_outline.created_at)}`
                    : "尚未生成大纲"}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <Button
                  onClick={() => {
                    void handleGenerateOutline();
                  }}
                  disabled={generationLocked}
                >
                  <BookOpenText className="h-4 w-4 mr-1" />
                  {snapshot.latest_outline ? "重新生成大纲" : "生成大纲"}
                </Button>
                <div className="rounded-md border bg-muted/20 p-3 text-sm whitespace-pre-wrap leading-6 max-h-96 overflow-y-auto">
                  {snapshot.latest_outline?.outline_markdown || "暂无大纲内容"}
                </div>
              </CardContent>
            </Card>
          )}

          {activeStage === "characters" && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">角色阶段</CardTitle>
                <CardDescription>生成后可在后续章节中自动引用角色关系。</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <Button
                  onClick={() => {
                    void handleGenerateCharacters();
                  }}
                  disabled={generationLocked}
                >
                  <Users className="h-4 w-4 mr-1" />
                  {snapshot.characters.length > 0 ? "更新角色阵列" : "生成角色阵列"}
                </Button>

                {snapshot.characters.length === 0 ? (
                  <div className="text-sm text-muted-foreground">暂无角色数据</div>
                ) : (
                  <div className="space-y-2 max-h-96 overflow-y-auto pr-1">
                    {snapshot.characters.map((character) => (
                      <div key={character.id} className="rounded-md border p-3">
                        <div className="text-sm font-medium">{character.name}</div>
                        <div className="text-xs text-muted-foreground mt-1">
                          {character.role_type} · 版本 {character.version}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          )}

          {activeStage === "chapters" && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">章节阶段（半自动循环）</CardTitle>
                <CardDescription>主动作会自动生成下一章并执行一致性检查。</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <Button
                  onClick={() => {
                    void handleGenerateNextChapter();
                  }}
                  disabled={generationLocked}
                >
                  <Sparkles className="h-4 w-4 mr-1" />
                  生成下一章（含自动检查）
                </Button>

                <div className="space-y-2">
                  <label className="text-sm font-medium">选择章节</label>
                  <select
                    value={selectedChapterId}
                    onChange={(event) => setSelectedChapterId(event.target.value)}
                    className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
                    disabled={snapshot.chapters.length === 0}
                  >
                    {snapshot.chapters.length === 0 ? (
                      <option value="">暂无章节</option>
                    ) : (
                      snapshot.chapters.map((chapter) => (
                        <option key={chapter.id} value={chapter.id}>
                          第 {chapter.chapter_no} 章 · {chapter.title}
                        </option>
                      ))
                    )}
                  </select>
                </div>

                <div className="rounded-md border bg-muted/20 p-3 text-sm whitespace-pre-wrap leading-6 max-h-96 overflow-y-auto">
                  {selectedChapter?.content || "暂无章节内容"}
                </div>
              </CardContent>
            </Card>
          )}

          {activeStage === "qa" && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">质检阶段</CardTitle>
                <CardDescription>一致性检查采用软门槛，告警不阻塞。</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <Button
                  variant="outline"
                  onClick={() => {
                    void handleCheckConsistencyForSelectedChapter();
                  }}
                  disabled={generationLocked}
                >
                  一致性检查
                </Button>

                {snapshot.latest_consistency ? (
                  <div className="space-y-2">
                    <div className="flex flex-wrap items-center gap-2">
                      <Badge
                        variant={
                          snapshot.latest_consistency.score < 60 ? "destructive" : "default"
                        }
                      >
                        评分 {snapshot.latest_consistency.score.toFixed(1)}
                      </Badge>
                      <span className="text-xs text-muted-foreground">
                        {formatDateTime(snapshot.latest_consistency.created_at)}
                      </span>
                    </div>
                    {snapshot.latest_consistency.issues.length === 0 ? (
                      <div className="text-sm text-muted-foreground">暂无一致性问题</div>
                    ) : (
                      <div className="space-y-2 max-h-80 overflow-y-auto pr-1">
                        {snapshot.latest_consistency.issues.map((issue, index) => (
                          <div key={`${issue.code}-${index}`} className="rounded-md border p-3">
                            <div className="text-sm font-medium flex items-center gap-2">
                              {issue.level === "error" ? (
                                <AlertTriangle className="h-4 w-4 text-red-600" />
                              ) : (
                                <CheckCircle2 className="h-4 w-4 text-amber-600" />
                              )}
                              {issue.code}
                            </div>
                            <div className="text-xs text-muted-foreground mt-1">{issue.message}</div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ) : (
                  <div className="text-sm text-muted-foreground">尚未执行一致性检查</div>
                )}
              </CardContent>
            </Card>
          )}

          {activeStage === "publish" && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">发布阶段</CardTitle>
                <CardDescription>发布入口已独立到“发布”标签页。</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="text-sm text-muted-foreground">
                  当前已有 {snapshot.chapters.length} 章可发布内容。
                  {snapshot.latest_consistency && (
                    <span>
                      最近一致性评分 {snapshot.latest_consistency.score.toFixed(1)}。
                    </span>
                  )}
                </div>
                <Button
                  onClick={() => {
                    toast.info("请切换到“发布”标签页继续");
                  }}
                >
                  前往发布页
                </Button>
              </CardContent>
            </Card>
          )}

          <Collapsible>
            <Card>
              <CardHeader>
                <CollapsibleTrigger asChild>
                  <Button variant="ghost" className="justify-between w-full px-0">
                    <span className="font-medium">高级操作</span>
                    <ChevronDown className="h-4 w-4" />
                  </Button>
                </CollapsibleTrigger>
                <CardDescription>重写、润色等非主路径动作</CardDescription>
              </CardHeader>
              <CollapsibleContent>
                <CardContent className="space-y-3">
                  <div className="grid gap-3 lg:grid-cols-2">
                    <div className="space-y-2">
                      <label className="text-sm font-medium">重写要求</label>
                      <Textarea
                        value={rewriteInstructions}
                        onChange={(event) => setRewriteInstructions(event.target.value)}
                        placeholder="例如：强化冲突节奏，保留人物关系"
                        rows={3}
                      />
                      <Button
                        variant="outline"
                        onClick={() => {
                          void handleRewriteChapter();
                        }}
                        disabled={generationLocked || !selectedChapterId}
                        className="w-full"
                      >
                        重写当前章节
                      </Button>
                    </div>

                    <div className="space-y-2">
                      <label className="text-sm font-medium">润色重点</label>
                      <Textarea
                        value={polishFocus}
                        onChange={(event) => setPolishFocus(event.target.value)}
                        placeholder="例如：对白自然度、叙事流畅度"
                        rows={3}
                      />
                      <Button
                        variant="outline"
                        onClick={() => {
                          void handlePolishChapter();
                        }}
                        disabled={generationLocked || !selectedChapterId}
                        className="w-full"
                      >
                        润色当前章节
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </CollapsibleContent>
            </Card>
          </Collapsible>
        </div>

        <div className="space-y-3 xl:col-span-2 2xl:col-span-1 min-w-0">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">流程上下文</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-sm">
              <div className="flex items-center justify-between gap-2">
                <span className="text-muted-foreground">目标字数</span>
                <span>{snapshot.project.target_words.toLocaleString("zh-CN")}</span>
              </div>
              <div className="flex items-center justify-between gap-2">
                <span className="text-muted-foreground">当前字数</span>
                <span>{snapshot.project.current_word_count.toLocaleString("zh-CN")}</span>
              </div>
              <div className="flex items-center justify-between gap-2">
                <span className="text-muted-foreground">章节</span>
                <span>{snapshot.chapters.length}</span>
              </div>
              <div className="flex items-center justify-between gap-2">
                <span className="text-muted-foreground">角色</span>
                <span>{snapshot.characters.length}</span>
              </div>
              {snapshot.latest_consistency && (
                <div className="flex items-center justify-between gap-2">
                  <span className="text-muted-foreground">最新质检</span>
                  <Badge
                    variant={
                      snapshot.latest_consistency.score < 60 ? "destructive" : "default"
                    }
                  >
                    {snapshot.latest_consistency.score.toFixed(1)}
                  </Badge>
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm">最近运行记录</CardTitle>
              <CardDescription>默认展示最近 3 条</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2">
              {loadingRuns ? (
                <div className="text-xs text-muted-foreground">加载中...</div>
              ) : runs.length === 0 ? (
                <div className="text-xs text-muted-foreground">暂无运行记录</div>
              ) : (
                runs.slice(0, 3).map((run) => (
                  <div key={run.id} className="rounded-md border p-2">
                    <div className="text-xs font-medium">{formatRunMode(run.mode)}</div>
                    <div className="text-[11px] text-muted-foreground mt-1">
                      {formatDateTime(run.created_at)}
                    </div>
                    <div className="mt-1">
                      <Badge variant={getRunStatusVariant(run.result_status)}>
                        {formatRunStatus(run.result_status)}
                      </Badge>
                    </div>
                  </div>
                ))
              )}
            </CardContent>
          </Card>

          {loadError && (
            <Card>
              <CardHeader>
                <CardTitle className="text-sm text-destructive">数据加载告警</CardTitle>
              </CardHeader>
              <CardContent className="text-xs text-destructive">{loadError}</CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}

export default NovelFlowWorkbench;
