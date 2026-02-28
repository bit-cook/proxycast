/**
 * @file NovelPublishTab.tsx
 * @description 小说项目发布 Tab，展示章节选择、平台配置与发布前检查
 * @module components/projects/tabs/NovelPublishTab
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";
import {
  getNovelProjectSnapshot,
  type NovelChapterRecord,
  type NovelProjectSnapshot,
} from "@/lib/api/novel";
import {
  AlertTriangle,
  CheckCircle2,
  Loader2,
  RefreshCw,
  SendIcon,
  XCircle,
} from "lucide-react";

export interface NovelPublishTabProps {
  /** 项目 ID */
  projectId: string;
}

type CheckLevel = "pass" | "warn" | "fail";

interface PreflightCheckItem {
  key: string;
  label: string;
  level: CheckLevel;
  detail: string;
}

interface PublishPlatformOption {
  id: string;
  name: string;
  description: string;
  icon: string;
}

const DEFAULT_SELECTED_PLATFORMS = ["fanqie", "qidian"];

const PUBLISH_PLATFORM_OPTIONS: PublishPlatformOption[] = [
  {
    id: "fanqie",
    name: "番茄小说",
    description: "番茄作家专区连载发布",
    icon: "🍅",
  },
  {
    id: "qidian",
    name: "起点小说",
    description: "起点作家专区连载发布",
    icon: "📚",
  },
  {
    id: "qimao",
    name: "七猫小说",
    description: "七猫作家中心连载发布",
    icon: "🐱",
  },
  {
    id: "jjwxc",
    name: "晋江文学城",
    description: "晋江作者后台连载发布",
    icon: "🌸",
  },
  {
    id: "faloo",
    name: "飞卢小说网",
    description: "飞卢作家后台连载发布",
    icon: "⚡",
  },
  {
    id: "zongheng",
    name: "纵横中文网",
    description: "纵横作家专区连载发布",
    icon: "🧭",
  },
  {
    id: "17k",
    name: "17K小说网",
    description: "17K作者后台连载发布",
    icon: "🔥",
  },
];

const MIN_CHAPTER_WORDS = 1000;

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "未知错误";
}

function formatDateTime(timestamp: number): string {
  return new Date(timestamp).toLocaleString("zh-CN", { hour12: false });
}

function formatChapterStatus(status: string): string {
  if (status === "draft") {
    return "草稿";
  }
  if (status === "published") {
    return "已发布";
  }
  return status;
}

function getChapterStatusVariant(
  status: string,
): "default" | "secondary" | "destructive" | "outline" {
  if (status === "published") {
    return "default";
  }
  if (status === "draft") {
    return "secondary";
  }
  return "outline";
}

function getCheckVariant(
  level: CheckLevel,
): "default" | "secondary" | "destructive" | "outline" {
  if (level === "pass") {
    return "default";
  }
  if (level === "warn") {
    return "secondary";
  }
  return "destructive";
}

export function NovelPublishTab({ projectId }: NovelPublishTabProps) {
  const [snapshot, setSnapshot] = useState<NovelProjectSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedChapterIds, setSelectedChapterIds] = useState<string[]>([]);
  const [selectedPlatforms, setSelectedPlatforms] = useState<string[]>(
    DEFAULT_SELECTED_PLATFORMS,
  );

  const loadSnapshot = useCallback(async () => {
    setLoading(true);
    try {
      const result = await getNovelProjectSnapshot(projectId);
      setSnapshot(result);
      setLoadError(null);
    } catch (error) {
      setSnapshot(null);
      setLoadError(getErrorMessage(error));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void loadSnapshot();
  }, [loadSnapshot]);

  useEffect(() => {
    if (!snapshot) {
      setSelectedChapterIds([]);
      return;
    }

    const validIds = new Set(snapshot.chapters.map((chapter) => chapter.id));
    setSelectedChapterIds((prev) => {
      const next = prev.filter((id) => validIds.has(id));
      if (next.length > 0) {
        return next;
      }
      const latestChapter = snapshot.chapters[snapshot.chapters.length - 1];
      return latestChapter ? [latestChapter.id] : [];
    });
  }, [snapshot]);

  const selectedChapters = useMemo<NovelChapterRecord[]>(() => {
    if (!snapshot) {
      return [];
    }
    const selectedSet = new Set(selectedChapterIds);
    return snapshot.chapters.filter((chapter) => selectedSet.has(chapter.id));
  }, [selectedChapterIds, snapshot]);

  const progressValue = useMemo(() => {
    if (!snapshot || snapshot.project.target_words <= 0) {
      return 0;
    }
    return Math.min(
      100,
      Math.round(
        (snapshot.project.current_word_count / snapshot.project.target_words) * 100,
      ),
    );
  }, [snapshot]);

  const preflightChecks = useMemo<PreflightCheckItem[]>(() => {
    const checks: PreflightCheckItem[] = [];
    checks.push({
      key: "chapter-selection",
      label: "已选择待发布章节",
      level: selectedChapters.length > 0 ? "pass" : "fail",
      detail:
        selectedChapters.length > 0
          ? `已选择 ${selectedChapters.length} 章`
          : "请至少选择 1 个章节",
    });
    checks.push({
      key: "platform-selection",
      label: "已选择发布平台",
      level: selectedPlatforms.length > 0 ? "pass" : "fail",
      detail:
        selectedPlatforms.length > 0
          ? `已选择 ${selectedPlatforms.length} 个平台`
          : "请至少选择 1 个平台",
    });

    const shortChapters = selectedChapters.filter(
      (chapter) => chapter.word_count < MIN_CHAPTER_WORDS,
    );
    checks.push({
      key: "chapter-length",
      label: "章节字数检查",
      level: shortChapters.length === 0 ? "pass" : "warn",
      detail:
        shortChapters.length === 0
          ? "章节字数达到建议阈值"
          : `有 ${shortChapters.length} 章低于 ${MIN_CHAPTER_WORDS} 字`,
    });

    const latestConsistency = snapshot?.latest_consistency;
    if (!latestConsistency) {
      checks.push({
        key: "consistency",
        label: "一致性检查",
        level: "warn",
        detail: "尚未执行一致性检查，建议发布前先检查",
      });
      return checks;
    }

    const consistencyScore = latestConsistency.score;
    const level: CheckLevel =
      consistencyScore >= 80 ? "pass" : consistencyScore >= 60 ? "warn" : "fail";
    checks.push({
      key: "consistency",
      label: "一致性检查",
      level,
      detail: `最新评分 ${consistencyScore.toFixed(1)}（${formatDateTime(latestConsistency.created_at)}）`,
    });
    return checks;
  }, [selectedChapters, selectedPlatforms, snapshot]);

  const hasBlockingFailure = preflightChecks.some((item) => item.level === "fail");

  const toggleChapter = useCallback((chapterId: string, checked: boolean) => {
    setSelectedChapterIds((prev) => {
      if (checked) {
        return prev.includes(chapterId) ? prev : [...prev, chapterId];
      }
      return prev.filter((id) => id !== chapterId);
    });
  }, []);

  const togglePlatform = useCallback((platformId: string, checked: boolean) => {
    setSelectedPlatforms((prev) => {
      if (checked) {
        return prev.includes(platformId) ? prev : [...prev, platformId];
      }
      return prev.filter((id) => id !== platformId);
    });
  }, []);

  return (
    <div className="p-4 space-y-4">
      <div className="flex items-center justify-between gap-2">
        <div>
          <h2 className="text-lg font-medium">小说发布</h2>
          <p className="text-sm text-muted-foreground">
            选择章节与平台，完成发布前检查
          </p>
        </div>
        <Button variant="outline" onClick={() => void loadSnapshot()} disabled={loading}>
          <RefreshCw className={cn("h-4 w-4 mr-1", loading && "animate-spin")} />
          刷新
        </Button>
      </div>

      {loading && !snapshot ? (
        <Card>
          <CardContent className="pt-6 flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            正在加载发布数据...
          </CardContent>
        </Card>
      ) : loadError ? (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">加载发布数据失败</CardTitle>
            <CardDescription>{loadError}</CardDescription>
          </CardHeader>
        </Card>
      ) : !snapshot ? (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">暂无可发布数据</CardTitle>
            <CardDescription>
              请先在内容页完成小说初始化，并生成章节后再发布。
            </CardDescription>
          </CardHeader>
        </Card>
      ) : (
        <>
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
            <Card>
              <CardHeader className="pb-2">
                <CardDescription>可选章节</CardDescription>
                <CardTitle className="text-xl">{snapshot.chapters.length}</CardTitle>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardDescription>已选章节</CardDescription>
                <CardTitle className="text-xl">{selectedChapters.length}</CardTitle>
              </CardHeader>
            </Card>
            <Card className="col-span-2">
              <CardHeader className="pb-2">
                <CardDescription>全书进度</CardDescription>
                <CardTitle className="text-xl">
                  {snapshot.project.current_word_count.toLocaleString("zh-CN")} /{" "}
                  {snapshot.project.target_words.toLocaleString("zh-CN")} 字
                </CardTitle>
              </CardHeader>
              <CardContent className="pt-0">
                <Progress value={progressValue} />
                <div className="mt-1 text-xs text-muted-foreground">
                  已完成 {progressValue}%
                </div>
              </CardContent>
            </Card>
          </div>

          <div className="grid gap-3 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="text-base">待发布章节</CardTitle>
                <CardDescription>默认已选最新章节，可多选批量发布</CardDescription>
              </CardHeader>
              <CardContent className="space-y-2">
                {snapshot.chapters.length === 0 ? (
                  <div className="text-sm text-muted-foreground">暂无章节可发布</div>
                ) : (
                  <div className="max-h-72 overflow-y-auto space-y-2 pr-1">
                    {snapshot.chapters.map((chapter) => {
                      const checked = selectedChapterIds.includes(chapter.id);
                      return (
                        <label
                          key={chapter.id}
                          className="flex items-start gap-3 rounded-md border p-3 cursor-pointer hover:bg-muted/30"
                        >
                          <Checkbox
                            checked={checked}
                            onCheckedChange={(value) =>
                              toggleChapter(chapter.id, Boolean(value))
                            }
                          />
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center justify-between gap-2">
                              <p className="text-sm font-medium truncate">
                                第 {chapter.chapter_no} 章 · {chapter.title}
                              </p>
                              <Badge variant={getChapterStatusVariant(chapter.status)}>
                                {formatChapterStatus(chapter.status)}
                              </Badge>
                            </div>
                            <p className="text-xs text-muted-foreground mt-1">
                              {chapter.word_count.toLocaleString("zh-CN")} 字 · 更新于{" "}
                              {formatDateTime(chapter.updated_at)}
                            </p>
                          </div>
                        </label>
                      );
                    })}
                  </div>
                )}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-base">发布平台</CardTitle>
                <CardDescription>当前只提供发布编排，平台连接后续接入</CardDescription>
              </CardHeader>
              <CardContent className="space-y-2">
                {PUBLISH_PLATFORM_OPTIONS.map((platform) => {
                  const checked = selectedPlatforms.includes(platform.id);
                  return (
                    <label
                      key={platform.id}
                      className="flex items-start gap-3 rounded-md border p-3 cursor-pointer hover:bg-muted/30"
                    >
                      <Checkbox
                        checked={checked}
                        onCheckedChange={(value) =>
                          togglePlatform(platform.id, Boolean(value))
                        }
                      />
                      <div className="min-w-0">
                        <div className="text-sm font-medium">
                          {platform.icon} {platform.name}
                        </div>
                        <div className="text-xs text-muted-foreground mt-1">
                          {platform.description}
                        </div>
                      </div>
                    </label>
                  );
                })}
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">发布前检查</CardTitle>
              <CardDescription>用于确认章节质量与发布条件</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2">
              {preflightChecks.map((item) => (
                <div
                  key={item.key}
                  className="flex items-start justify-between gap-3 rounded-md border p-3"
                >
                  <div className="min-w-0">
                    <div className="text-sm font-medium flex items-center gap-2">
                      {item.level === "pass" ? (
                        <CheckCircle2 className="h-4 w-4 text-emerald-600" />
                      ) : item.level === "warn" ? (
                        <AlertTriangle className="h-4 w-4 text-amber-600" />
                      ) : (
                        <XCircle className="h-4 w-4 text-red-600" />
                      )}
                      {item.label}
                    </div>
                    <div className="text-xs text-muted-foreground mt-1">
                      {item.detail}
                    </div>
                  </div>
                  <Badge variant={getCheckVariant(item.level)}>
                    {item.level === "pass"
                      ? "通过"
                      : item.level === "warn"
                        ? "警告"
                        : "阻塞"}
                  </Badge>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">执行发布</CardTitle>
              <CardDescription>
                当前版本尚未接入小说发布后端接口，此处先展示发布编排状态。
              </CardDescription>
            </CardHeader>
            <CardContent className="flex items-center justify-between gap-3">
              <div className="text-sm text-muted-foreground">
                {hasBlockingFailure
                  ? "存在阻塞项，请先修复后再发布。"
                  : "发布条件已满足，等待发布接口接入。"}
              </div>
              <Button disabled>
                <SendIcon className="h-4 w-4 mr-1" />
                发布（即将支持）
              </Button>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}

export default NovelPublishTab;
