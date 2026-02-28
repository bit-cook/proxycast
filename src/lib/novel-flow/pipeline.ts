import type { NovelProjectSnapshot } from "@/lib/api/novel";
import type { NovelSettingsEnvelope } from "@/lib/novel-settings/types";

export type NovelPipelineStage =
  | "setup"
  | "outline"
  | "characters"
  | "chapters"
  | "qa"
  | "publish";

export type NovelStageStatus = "locked" | "ready" | "done" | "warning";

export type NovelPrimaryActionKey =
  | "save-settings"
  | "generate-outline"
  | "generate-characters"
  | "generate-next-chapter"
  | "run-consistency"
  | "open-publish";

export interface NovelPipelinePrimaryAction {
  key: NovelPrimaryActionKey;
  label: string;
  disabled?: boolean;
}

export interface NovelPipelineState {
  currentStage: NovelPipelineStage;
  stageStatus: Record<NovelPipelineStage, NovelStageStatus>;
  primaryAction: NovelPipelinePrimaryAction;
}

function isSetupReady(settings: NovelSettingsEnvelope | null): boolean {
  if (!settings) {
    return false;
  }

  const { data } = settings;
  return (
    data.genres.length > 0 &&
    data.oneLinePitch.trim().length > 0 &&
    data.mainCharacter.name.trim().length > 0
  );
}

function getCurrentStage(
  stageStatus: Record<NovelPipelineStage, NovelStageStatus>,
): NovelPipelineStage {
  const order: NovelPipelineStage[] = [
    "setup",
    "outline",
    "characters",
    "chapters",
    "qa",
    "publish",
  ];

  for (const stage of order) {
    if (stageStatus[stage] === "ready" || stageStatus[stage] === "warning") {
      return stage;
    }
  }

  return "publish";
}

function getPrimaryAction(
  stage: NovelPipelineStage,
  stageStatus: Record<NovelPipelineStage, NovelStageStatus>,
): NovelPipelinePrimaryAction {
  switch (stage) {
    case "setup":
      return {
        key: "save-settings",
        label: "保存创作设定",
        disabled: stageStatus.setup === "locked",
      };
    case "outline":
      return {
        key: "generate-outline",
        label: stageStatus.outline === "done" ? "重新生成大纲" : "生成大纲",
        disabled: stageStatus.outline === "locked",
      };
    case "characters":
      return {
        key: "generate-characters",
        label:
          stageStatus.characters === "done" ? "更新角色阵列" : "生成角色阵列",
        disabled: stageStatus.characters === "locked",
      };
    case "chapters":
      return {
        key: "generate-next-chapter",
        label: "生成下一章（含自动检查）",
        disabled: stageStatus.chapters === "locked",
      };
    case "qa":
      return {
        key: "run-consistency",
        label: "执行一致性检查",
        disabled: stageStatus.qa === "locked",
      };
    case "publish":
      return {
        key: "open-publish",
        label: "前往发布页",
        disabled: stageStatus.publish === "locked",
      };
    default:
      return {
        key: "save-settings",
        label: "保存创作设定",
      };
  }
}

export function resolveNovelPipelineState(params: {
  snapshot: NovelProjectSnapshot | null;
  settings: NovelSettingsEnvelope | null;
}): NovelPipelineState {
  const { snapshot, settings } = params;

  const setupDone = isSetupReady(settings);
  const hasOutline = Boolean(snapshot?.latest_outline);
  const hasCharacters = (snapshot?.characters.length ?? 0) > 0;
  const hasChapters = (snapshot?.chapters.length ?? 0) > 0;

  const latestConsistency = snapshot?.latest_consistency;
  const consistencyScore = latestConsistency?.score;

  const stageStatus: Record<NovelPipelineStage, NovelStageStatus> = {
    setup: setupDone ? "done" : "ready",
    outline: setupDone ? (hasOutline ? "done" : "ready") : "locked",
    characters:
      setupDone && hasOutline
        ? hasCharacters
          ? "done"
          : "ready"
        : "locked",
    chapters:
      setupDone && hasOutline && hasCharacters
        ? hasChapters
          ? "done"
          : "ready"
        : "locked",
    qa:
      setupDone && hasOutline && hasCharacters && hasChapters
        ? consistencyScore === undefined
          ? "ready"
          : consistencyScore < 60
            ? "warning"
            : "done"
        : "locked",
    publish:
      setupDone && hasOutline && hasCharacters && hasChapters ? "ready" : "locked",
  };

  const currentStage = getCurrentStage(stageStatus);

  return {
    currentStage,
    stageStatus,
    primaryAction: getPrimaryAction(currentStage, stageStatus),
  };
}
