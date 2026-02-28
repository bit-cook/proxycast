import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import {
  ALL_NOVEL_GENRES,
  createEmptyAntagonist,
  createEmptyPlotBeat,
  createEmptyReferenceWork,
  createEmptySideCharacter,
  createEmptyTaboo,
  type Antagonist,
  type NovelSettingsV1,
  type PlotBeat,
  type SideCharacter,
} from "@/lib/novel-settings/types";
import { CheckCircle2, ChevronRight, Plus, Trash2 } from "lucide-react";

const NARRATION_OPTIONS = ["第一人称", "第三人称有限", "全知视角"] as const;
const TONE_OPTIONS = [
  "热血",
  "黑暗",
  "轻松",
  "细腻",
  "幽默",
  "压抑",
  "爽快",
  "文艺",
  "写实",
] as const;
const CHEAT_OPTIONS = ["无敌流", "稳步成长", "真实吃力", "反转流", "废柴逆袭"] as const;
const FOCUS_OPTIONS = ["战斗", "感情", "智斗", "日常", "装逼", "后宫", "权谋", "经营"] as const;
const ENDING_OPTIONS = ["HE", "BE", "开放", "大团圆", "虐", "爽", "开放式"] as const;
const RELATIONSHIP_OPTIONS = [
  "盟友",
  "恋人",
  "导师",
  "死敌",
  "家人",
  "竞争者",
  "炮灰",
  "同门",
  "队友",
  "上司",
  "下属",
  "其他",
] as const;
const ARC_OPTIONS = ["成长", "黑化", "救赎", "牺牲", "退场", "保持不变", "其他"] as const;

type WizardStepId =
  | "basic"
  | "world"
  | "plot"
  | "characters"
  | "style"
  | "constraints";

interface WizardStepMeta {
  id: WizardStepId;
  title: string;
  description: string;
}

const WIZARD_STEPS: WizardStepMeta[] = [
  { id: "basic", title: "基础与主角", description: "题材、简介、主角核心信息" },
  { id: "world", title: "世界观", description: "背景、冲突、规则与文化" },
  { id: "plot", title: "剧情骨架", description: "开头、关键节点、副线与结局" },
  { id: "characters", title: "角色关系", description: "配角和反派阵列" },
  { id: "style", title: "风格与规模", description: "写作风格、字数、开关" },
  { id: "constraints", title: "约束与参考", description: "禁忌与参考作品" },
];

interface CharacterCore {
  id: string;
  name: string;
  nickname: string;
  gender: string;
  age: string;
  relationship: string;
  relationshipCustom: string;
  personalityTags: string[];
  background: string;
  abilities: string;
  role: string;
  arc: string;
  arcCustom: string;
}

export interface NovelSettingsWizardProps {
  value: NovelSettingsV1;
  onChange: (next: NovelSettingsV1) => void;
  disabled?: boolean;
  onBeforeStepChange?: () => Promise<boolean>;
}

function updateById<T extends { id: string }>(
  list: T[],
  id: string,
  updater: (item: T) => T,
): T[] {
  return list.map((item) => (item.id === id ? updater(item) : item));
}

function removeById<T extends { id: string }>(list: T[], id: string): T[] {
  return list.filter((item) => item.id !== id);
}

function parseTags(value: string): string[] {
  return value
    .split(/[,，、\s]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function isBeatFilled(list: PlotBeat[]): boolean {
  return list.some((item) => item.title.trim() || item.detail.trim());
}

function isCharacterFilled(list: SideCharacter[] | Antagonist[]): boolean {
  return list.some((item) => item.name.trim());
}

function getStepCompleted(step: WizardStepId, value: NovelSettingsV1): boolean {
  switch (step) {
    case "basic":
      return (
        value.genres.length > 0 &&
        value.oneLinePitch.trim().length > 0 &&
        value.mainCharacter.name.trim().length > 0
      );
    case "world":
      return (
        value.worldSummary.trim().length > 0 &&
        value.conflictTheme.trim().length > 0 &&
        [
          value.worldDetails.powerSystem,
          value.worldDetails.factions,
          value.worldDetails.historyEvents,
          value.worldDetails.importantLocations,
          value.worldDetails.cultureAndTaboos,
        ].some((item) => item.trim().length > 0)
      );
    case "plot":
      return (
        value.opening.trim().length > 0 &&
        isBeatFilled(value.middleBeats) &&
        value.endingType.trim().length > 0
      );
    case "characters":
      return isCharacterFilled(value.sideCharacters) || isCharacterFilled(value.antagonists);
    case "style":
      return (
        value.totalWords > 0 &&
        value.chapterWords > 0 &&
        value.writingStyle.narration.trim().length > 0
      );
    case "constraints":
      return (
        value.taboos.some((item) => item.content.trim()) ||
        value.references.some((item) => item.title.trim() || item.inspiration.trim())
      );
    default:
      return false;
  }
}

function buildStepSummary(step: WizardStepId, value: NovelSettingsV1): string[] {
  switch (step) {
    case "basic": {
      return [
        `题材: ${value.genres.length > 0 ? value.genres.join("、") : "未填写"}`,
        `简介: ${value.oneLinePitch.trim() || "未填写"}`,
        `主角: ${value.mainCharacter.name.trim() || "未填写"}`,
      ];
    }
    case "world": {
      return [
        `世界观: ${value.worldSummary.trim() || "未填写"}`,
        `核心冲突: ${value.conflictTheme.trim() || "未填写"}`,
        `体系: ${value.worldDetails.powerSystem.trim() || "未填写"}`,
      ];
    }
    case "plot": {
      return [
        `开头: ${value.opening.trim() || "未填写"}`,
        `中段节点: ${value.middleBeats.length} 个`,
        `副线: ${value.subplots.length} 条`,
        `结局: ${value.endingType || "未填写"}`,
      ];
    }
    case "characters": {
      return [
        `配角: ${value.sideCharacters.length} 人`,
        `反派: ${value.antagonists.length} 人`,
      ];
    }
    case "style": {
      return [
        `视角: ${value.writingStyle.narration}`,
        `语气: ${value.writingStyle.tones.join("、") || "未填写"}`,
        `字数: 全书 ${value.totalWords} / 每章 ${value.chapterWords}`,
      ];
    }
    case "constraints": {
      return [
        `禁忌: ${value.taboos.filter((item) => item.content.trim()).length} 条`,
        `参考: ${value.references.filter((item) => item.title.trim() || item.inspiration.trim()).length} 条`,
      ];
    }
    default:
      return ["未填写"];
  }
}

function CharacterCoreEditor<T extends CharacterCore>({
  value,
  onChange,
  disabled,
}: {
  value: T;
  onChange: (next: T) => void;
  disabled?: boolean;
}) {
  const patch = (partial: Partial<T>) => onChange({ ...value, ...partial });

  return (
    <div className="space-y-3">
      <div className="grid gap-2 md:grid-cols-2">
        <Input
          placeholder="姓名"
          value={value.name}
          onChange={(event) => patch({ name: event.target.value } as Partial<T>)}
          disabled={disabled}
        />
        <Input
          placeholder="昵称"
          value={value.nickname}
          onChange={(event) => patch({ nickname: event.target.value } as Partial<T>)}
          disabled={disabled}
        />
      </div>

      <div className="grid gap-2 md:grid-cols-3">
        <select
          value={value.gender}
          onChange={(event) => patch({ gender: event.target.value } as Partial<T>)}
          disabled={disabled}
          className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
        >
          <option value="男">男</option>
          <option value="女">女</option>
          <option value="其他">其他</option>
        </select>
        <Input
          placeholder="年龄"
          value={value.age}
          onChange={(event) => patch({ age: event.target.value } as Partial<T>)}
          disabled={disabled}
        />
        <select
          value={value.relationship || "其他"}
          onChange={(event) => {
            const next = event.target.value;
            patch({
              relationship: next === "其他" ? "" : next,
              relationshipCustom: next === "其他" ? value.relationshipCustom : "",
            } as Partial<T>);
          }}
          disabled={disabled}
          className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
        >
          {RELATIONSHIP_OPTIONS.map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
      </div>

      {!value.relationship ? (
        <Input
          placeholder="自定义关系"
          value={value.relationshipCustom}
          onChange={(event) =>
            patch({ relationshipCustom: event.target.value } as Partial<T>)
          }
          disabled={disabled}
        />
      ) : null}

      <div className="grid gap-2 md:grid-cols-2">
        <Input
          placeholder="性格标签（逗号分隔）"
          value={value.personalityTags.join("、")}
          onChange={(event) =>
            patch({ personalityTags: parseTags(event.target.value) } as Partial<T>)
          }
          disabled={disabled}
        />
        <select
          value={value.arc || "其他"}
          onChange={(event) => {
            const next = event.target.value;
            patch({
              arc: next === "其他" ? "" : next,
              arcCustom: next === "其他" ? value.arcCustom : "",
            } as Partial<T>);
          }}
          disabled={disabled}
          className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
        >
          {ARC_OPTIONS.map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
      </div>

      {!value.arc ? (
        <Input
          placeholder="自定义人物弧光"
          value={value.arcCustom}
          onChange={(event) => patch({ arcCustom: event.target.value } as Partial<T>)}
          disabled={disabled}
        />
      ) : null}

      <Input
        placeholder="身份背景"
        value={value.background}
        onChange={(event) => patch({ background: event.target.value } as Partial<T>)}
        disabled={disabled}
      />

      <Textarea
        placeholder="能力/弱点"
        rows={2}
        value={value.abilities}
        onChange={(event) => patch({ abilities: event.target.value } as Partial<T>)}
        disabled={disabled}
      />

      <Textarea
        placeholder="故事作用"
        rows={2}
        value={value.role}
        onChange={(event) => patch({ role: event.target.value } as Partial<T>)}
        disabled={disabled}
      />
    </div>
  );
}

function PlotBeatList({
  title,
  list,
  addText,
  onChange,
  disabled,
}: {
  title: string;
  list: PlotBeat[];
  addText: string;
  onChange: (next: PlotBeat[]) => void;
  disabled?: boolean;
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label>{title}</Label>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => onChange([...list, createEmptyPlotBeat()])}
          disabled={disabled}
        >
          <Plus className="h-3 w-3 mr-1" />
          {addText}
        </Button>
      </div>

      {list.length === 0 ? (
        <p className="text-xs text-muted-foreground">暂未填写</p>
      ) : (
        <div className="space-y-2">
          {list.map((item, index) => (
            <div key={item.id} className="rounded-md border p-2 space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-xs text-muted-foreground">{index + 1}.</span>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => onChange(removeById(list, item.id))}
                  disabled={disabled}
                  className="h-6 w-6 text-destructive"
                >
                  <Trash2 className="h-3 w-3" />
                </Button>
              </div>
              <Input
                placeholder="标题"
                value={item.title}
                onChange={(event) =>
                  onChange(
                    updateById(list, item.id, (beat) => ({
                      ...beat,
                      title: event.target.value,
                    })),
                  )
                }
                disabled={disabled}
              />
              <Textarea
                rows={2}
                placeholder="描述"
                value={item.detail}
                onChange={(event) =>
                  onChange(
                    updateById(list, item.id, (beat) => ({
                      ...beat,
                      detail: event.target.value,
                    })),
                  )
                }
                disabled={disabled}
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function NovelSettingsWizard({
  value,
  onChange,
  disabled,
  onBeforeStepChange,
}: NovelSettingsWizardProps) {
  const [stepIndex, setStepIndex] = useState(0);
  const [navigating, setNavigating] = useState(false);
  const [showStepNav, setShowStepNav] = useState(true);

  const currentStep = WIZARD_STEPS[stepIndex];
  const stepDoneMap = useMemo(() => {
    const entries = WIZARD_STEPS.map((step) => [step.id, getStepCompleted(step.id, value)]);
    return Object.fromEntries(entries) as Record<WizardStepId, boolean>;
  }, [value]);

  const doneCount = useMemo(
    () => Object.values(stepDoneMap).filter(Boolean).length,
    [stepDoneMap],
  );

  const progressPercent = Math.round((doneCount / WIZARD_STEPS.length) * 100);

  const patch = (partial: Partial<NovelSettingsV1>) => onChange({ ...value, ...partial });
  const patchWorldDetails = (partial: Partial<NovelSettingsV1["worldDetails"]>) =>
    patch({ worldDetails: { ...value.worldDetails, ...partial } });
  const patchWritingStyle = (partial: Partial<NovelSettingsV1["writingStyle"]>) =>
    patch({ writingStyle: { ...value.writingStyle, ...partial } });

  const toggleTag = (list: string[], target: string): string[] =>
    list.includes(target) ? list.filter((item) => item !== target) : [...list, target];

  const tryNavigateTo = async (targetIndex: number) => {
    if (targetIndex < 0 || targetIndex >= WIZARD_STEPS.length || targetIndex === stepIndex) {
      return;
    }

    if (onBeforeStepChange) {
      setNavigating(true);
      try {
        const ok = await onBeforeStepChange();
        if (!ok) {
          return;
        }
      } finally {
        setNavigating(false);
      }
    }

    setStepIndex(targetIndex);
  };

  const stepSummaryLines = buildStepSummary(currentStep.id, value);
  const lockUI = disabled || navigating;

  return (
    <div className="space-y-3">
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between gap-3">
            <div>
              <CardTitle className="text-base">向导模式</CardTitle>
              <CardDescription>按步骤完善设定，减少长屏记忆负担。</CardDescription>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="outline">完成度 {progressPercent}%</Badge>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setShowStepNav((prev) => !prev)}
              >
                {showStepNav ? "收起步骤" : "展开步骤"}
              </Button>
            </div>
          </div>
          <div className="h-2 rounded-full bg-muted overflow-hidden mt-2">
            <div
              className="h-full bg-primary transition-all"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
        </CardHeader>
      </Card>

      <div
        className={cn(
          "grid gap-3",
          showStepNav ? "xl:grid-cols-[220px_minmax(0,1fr)]" : "xl:grid-cols-1",
        )}
      >
        {showStepNav && (
          <Card className="min-w-0">
            <CardHeader>
              <CardTitle className="text-sm">步骤导航</CardTitle>
            </CardHeader>
            <CardContent className="space-y-1">
              {WIZARD_STEPS.map((step, index) => {
                const done = stepDoneMap[step.id];
                const active = index === stepIndex;
                return (
                  <button
                    key={step.id}
                    type="button"
                    onClick={() => {
                      void tryNavigateTo(index);
                    }}
                    disabled={lockUI}
                    className={cn(
                      "w-full rounded-md border px-3 py-2 text-left",
                      active && "border-primary bg-primary/5",
                      lockUI && "opacity-70",
                    )}
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-sm font-medium">
                        {index + 1}. {step.title}
                      </span>
                      {done ? (
                        <CheckCircle2 className="h-4 w-4 text-emerald-600" />
                      ) : (
                        <ChevronRight className="h-4 w-4 text-muted-foreground" />
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">{step.description}</p>
                  </button>
                );
              })}
            </CardContent>
          </Card>
        )}

        <Card className="min-w-0">
          <CardHeader>
            <CardTitle className="text-base">{currentStep.title}</CardTitle>
            <CardDescription>{currentStep.description}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="rounded-md border bg-muted/20 p-3">
              <p className="text-xs font-medium text-muted-foreground mb-2">本步要点</p>
              <div className="space-y-1">
                {stepSummaryLines.map((line, index) => (
                  <p
                    key={`${currentStep.id}-${index}`}
                    className="text-sm text-muted-foreground"
                  >
                    {line}
                  </p>
                ))}
              </div>
            </div>

            {currentStep.id === "basic" ? (
              <>
                <div className="space-y-2">
                  <Label>小说类型（可多选）</Label>
                  <div className="flex flex-wrap gap-2">
                    {ALL_NOVEL_GENRES.map((genre) => {
                      const active = value.genres.includes(genre);
                      return (
                        <Badge
                          key={genre}
                          variant={active ? "default" : "outline"}
                          className={cn("cursor-pointer", lockUI && "pointer-events-none opacity-70")}
                          onClick={() => {
                            if (lockUI) {
                              return;
                            }
                            patch({
                              genres: active
                                ? value.genres.filter((item) => item !== genre)
                                : [...value.genres, genre],
                            });
                          }}
                        >
                          {genre}
                        </Badge>
                      );
                    })}
                  </div>
                </div>

                <Textarea
                  rows={2}
                  placeholder="一句话简介"
                  value={value.oneLinePitch}
                  onChange={(event) => patch({ oneLinePitch: event.target.value })}
                  disabled={lockUI}
                />

                <div className="grid gap-2 md:grid-cols-2">
                  <Input
                    placeholder="主角姓名"
                    value={value.mainCharacter.name}
                    onChange={(event) =>
                      patch({
                        mainCharacter: { ...value.mainCharacter, name: event.target.value },
                      })
                    }
                    disabled={lockUI}
                  />
                  <select
                    value={value.mainCharacter.gender}
                    onChange={(event) =>
                      patch({
                        mainCharacter: { ...value.mainCharacter, gender: event.target.value },
                      })
                    }
                    disabled={lockUI}
                    className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
                  >
                    <option value="男">男</option>
                    <option value="女">女</option>
                    <option value="其他">其他</option>
                  </select>
                </div>

                <div className="grid gap-2 md:grid-cols-2">
                  <Input
                    placeholder="年龄"
                    value={value.mainCharacter.age}
                    onChange={(event) =>
                      patch({
                        mainCharacter: { ...value.mainCharacter, age: event.target.value },
                      })
                    }
                    disabled={lockUI}
                  />
                  <Input
                    placeholder="性格"
                    value={value.mainCharacter.personality}
                    onChange={(event) =>
                      patch({
                        mainCharacter: {
                          ...value.mainCharacter,
                          personality: event.target.value,
                        },
                      })
                    }
                    disabled={lockUI}
                  />
                </div>
              </>
            ) : null}

            {currentStep.id === "world" ? (
              <>
                <Textarea
                  rows={3}
                  placeholder="世界观/时代背景"
                  value={value.worldSummary}
                  onChange={(event) => patch({ worldSummary: event.target.value })}
                  disabled={lockUI}
                />
                <Input
                  placeholder="核心冲突/主题"
                  value={value.conflictTheme}
                  onChange={(event) => patch({ conflictTheme: event.target.value })}
                  disabled={lockUI}
                />
                <Textarea
                  rows={2}
                  placeholder="力量体系"
                  value={value.worldDetails.powerSystem}
                  onChange={(event) => patchWorldDetails({ powerSystem: event.target.value })}
                  disabled={lockUI}
                />
                <Textarea
                  rows={2}
                  placeholder="势力分布"
                  value={value.worldDetails.factions}
                  onChange={(event) => patchWorldDetails({ factions: event.target.value })}
                  disabled={lockUI}
                />
                <Textarea
                  rows={2}
                  placeholder="历史事件"
                  value={value.worldDetails.historyEvents}
                  onChange={(event) => patchWorldDetails({ historyEvents: event.target.value })}
                  disabled={lockUI}
                />
                <Textarea
                  rows={2}
                  placeholder="重要地点"
                  value={value.worldDetails.importantLocations}
                  onChange={(event) => patchWorldDetails({ importantLocations: event.target.value })}
                  disabled={lockUI}
                />
                <Textarea
                  rows={2}
                  placeholder="文化与禁忌"
                  value={value.worldDetails.cultureAndTaboos}
                  onChange={(event) => patchWorldDetails({ cultureAndTaboos: event.target.value })}
                  disabled={lockUI}
                />
              </>
            ) : null}

            {currentStep.id === "plot" ? (
              <>
                <Textarea
                  rows={3}
                  placeholder="开头（前30%）"
                  value={value.opening}
                  onChange={(event) => patch({ opening: event.target.value })}
                  disabled={lockUI}
                />
                <PlotBeatList
                  title="中段高潮与关键转折"
                  list={value.middleBeats}
                  addText="添加节点"
                  onChange={(next) => patch({ middleBeats: next })}
                  disabled={lockUI}
                />
                <PlotBeatList
                  title="主要副线"
                  list={value.subplots}
                  addText="添加副线"
                  onChange={(next) => patch({ subplots: next })}
                  disabled={lockUI}
                />
                <select
                  value={value.endingType || ""}
                  onChange={(event) => patch({ endingType: event.target.value })}
                  disabled={lockUI}
                  className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
                >
                  <option value="">选择结局类型</option>
                  {ENDING_OPTIONS.map((ending) => (
                    <option key={ending} value={ending}>
                      {ending}
                    </option>
                  ))}
                </select>
              </>
            ) : null}

            {currentStep.id === "characters" ? (
              <>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>配角</Label>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() =>
                        patch({ sideCharacters: [...value.sideCharacters, createEmptySideCharacter()] })
                      }
                      disabled={lockUI}
                    >
                      <Plus className="h-3 w-3 mr-1" />
                      添加配角
                    </Button>
                  </div>
                  {value.sideCharacters.length === 0 ? (
                    <p className="text-xs text-muted-foreground">暂未添加配角</p>
                  ) : (
                    <div className="space-y-2">
                      {value.sideCharacters.map((character, index) => (
                        <div key={character.id} className="rounded-md border p-3 space-y-3">
                          <div className="flex items-center justify-between">
                            <span className="text-sm font-medium">配角 {index + 1}</span>
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              onClick={() =>
                                patch({ sideCharacters: removeById(value.sideCharacters, character.id) })
                              }
                              disabled={lockUI}
                              className="h-7 w-7 text-destructive"
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                          <CharacterCoreEditor
                            value={character}
                            onChange={(next) =>
                              patch({
                                sideCharacters: updateById(value.sideCharacters, character.id, () => next),
                              })
                            }
                            disabled={lockUI}
                          />
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>反派</Label>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() =>
                        patch({ antagonists: [...value.antagonists, createEmptyAntagonist()] })
                      }
                      disabled={lockUI}
                    >
                      <Plus className="h-3 w-3 mr-1" />
                      添加反派
                    </Button>
                  </div>
                  {value.antagonists.length === 0 ? (
                    <p className="text-xs text-muted-foreground">暂未添加反派</p>
                  ) : (
                    <div className="space-y-2">
                      {value.antagonists.map((character, index) => (
                        <div key={character.id} className="rounded-md border p-3 space-y-3">
                          <div className="flex items-center justify-between">
                            <span className="text-sm font-medium">反派 {index + 1}</span>
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              onClick={() =>
                                patch({ antagonists: removeById(value.antagonists, character.id) })
                              }
                              disabled={lockUI}
                              className="h-7 w-7 text-destructive"
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                          <CharacterCoreEditor
                            value={character}
                            onChange={(next) =>
                              patch({
                                antagonists: updateById(value.antagonists, character.id, () => next),
                              })
                            }
                            disabled={lockUI}
                          />
                          <Input
                            placeholder="反派动机"
                            value={character.motive}
                            onChange={(event) =>
                              patch({
                                antagonists: updateById(value.antagonists, character.id, (item) => ({
                                  ...item,
                                  motive: event.target.value,
                                })),
                              })
                            }
                            disabled={lockUI}
                          />
                          <Input
                            placeholder="最终下场"
                            value={character.fate}
                            onChange={(event) =>
                              patch({
                                antagonists: updateById(value.antagonists, character.id, (item) => ({
                                  ...item,
                                  fate: event.target.value,
                                })),
                              })
                            }
                            disabled={lockUI}
                          />
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </>
            ) : null}

            {currentStep.id === "style" ? (
              <>
                <div className="grid gap-2 md:grid-cols-2">
                  <select
                    value={value.writingStyle.narration}
                    onChange={(event) => patchWritingStyle({ narration: event.target.value })}
                    disabled={lockUI}
                    className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
                  >
                    {NARRATION_OPTIONS.map((option) => (
                      <option key={option} value={option}>
                        {option}
                      </option>
                    ))}
                  </select>
                  <select
                    value={value.writingStyle.cheatLevel}
                    onChange={(event) => patchWritingStyle({ cheatLevel: event.target.value })}
                    disabled={lockUI}
                    className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
                  >
                    {CHEAT_OPTIONS.map((option) => (
                      <option key={option} value={option}>
                        {option}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="space-y-2">
                  <Label>语气（可多选）</Label>
                  <div className="flex flex-wrap gap-2">
                    {TONE_OPTIONS.map((tone) => {
                      const active = value.writingStyle.tones.includes(tone);
                      return (
                        <Badge
                          key={tone}
                          variant={active ? "default" : "outline"}
                          className={cn("cursor-pointer", lockUI && "pointer-events-none opacity-70")}
                          onClick={() => {
                            if (lockUI) {
                              return;
                            }
                            patchWritingStyle({
                              tones: toggleTag(value.writingStyle.tones, tone),
                            });
                          }}
                        >
                          {tone}
                        </Badge>
                      );
                    })}
                  </div>
                </div>

                <div className="space-y-2">
                  <Label>重点描写（可多选）</Label>
                  <div className="flex flex-wrap gap-2">
                    {FOCUS_OPTIONS.map((focus) => {
                      const active = value.writingStyle.focusAreas.includes(focus);
                      return (
                        <Badge
                          key={focus}
                          variant={active ? "default" : "outline"}
                          className={cn("cursor-pointer", lockUI && "pointer-events-none opacity-70")}
                          onClick={() => {
                            if (lockUI) {
                              return;
                            }
                            patchWritingStyle({
                              focusAreas: toggleTag(value.writingStyle.focusAreas, focus),
                            });
                          }}
                        >
                          {focus}
                        </Badge>
                      );
                    })}
                  </div>
                </div>

                <div className="grid gap-2 md:grid-cols-3">
                  <Input
                    type="number"
                    min={1000}
                    step={1000}
                    value={value.totalWords}
                    onChange={(event) => patch({ totalWords: Number(event.target.value || 0) })}
                    disabled={lockUI}
                  />
                  <Input
                    type="number"
                    min={500}
                    step={500}
                    value={value.chapterWords}
                    onChange={(event) => {
                      const chapterWords = Number(event.target.value || 0);
                      patch({
                        chapterWords,
                        writingStyle: {
                          ...value.writingStyle,
                          wordsPerChapter: chapterWords,
                        },
                      });
                    }}
                    disabled={lockUI}
                  />
                  <Input
                    type="number"
                    min={0}
                    max={1}
                    step={0.05}
                    value={value.writingStyle.temperature}
                    onChange={(event) =>
                      patchWritingStyle({ temperature: Number(event.target.value || 0) })
                    }
                    disabled={lockUI}
                  />
                </div>

                <div className="grid gap-2 md:grid-cols-3 text-sm">
                  <label className="flex items-center justify-between rounded-md border px-3 py-2">
                    <span>NSFW</span>
                    <input
                      type="checkbox"
                      checked={value.nsfw}
                      onChange={(event) => patch({ nsfw: event.target.checked })}
                      disabled={lockUI}
                      className="h-4 w-4"
                    />
                  </label>
                  <label className="flex items-center justify-between rounded-md border px-3 py-2">
                    <span>系统文</span>
                    <input
                      type="checkbox"
                      checked={value.systemNovel}
                      onChange={(event) => patch({ systemNovel: event.target.checked })}
                      disabled={lockUI}
                      className="h-4 w-4"
                    />
                  </label>
                  <label className="flex items-center justify-between rounded-md border px-3 py-2">
                    <span>后宫</span>
                    <input
                      type="checkbox"
                      checked={value.harem}
                      onChange={(event) => patch({ harem: event.target.checked })}
                      disabled={lockUI}
                      className="h-4 w-4"
                    />
                  </label>
                </div>
              </>
            ) : null}

            {currentStep.id === "constraints" ? (
              <>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>禁忌列表</Label>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => patch({ taboos: [...value.taboos, createEmptyTaboo()] })}
                      disabled={lockUI}
                    >
                      <Plus className="h-3 w-3 mr-1" />
                      添加禁忌
                    </Button>
                  </div>
                  {value.taboos.length === 0 ? (
                    <p className="text-xs text-muted-foreground">暂未填写禁忌</p>
                  ) : (
                    <div className="space-y-2">
                      {value.taboos.map((taboo, index) => (
                        <div key={taboo.id} className="rounded-md border p-2 space-y-2">
                          <div className="flex items-center justify-between">
                            <span className="text-xs text-muted-foreground">{index + 1}.</span>
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              className="h-6 w-6 text-destructive"
                              onClick={() => patch({ taboos: removeById(value.taboos, taboo.id) })}
                              disabled={lockUI}
                            >
                              <Trash2 className="h-3 w-3" />
                            </Button>
                          </div>
                          <Textarea
                            rows={2}
                            placeholder="禁忌内容"
                            value={taboo.content}
                            onChange={(event) =>
                              patch({
                                taboos: updateById(value.taboos, taboo.id, (item) => ({
                                  ...item,
                                  content: event.target.value,
                                })),
                              })
                            }
                            disabled={lockUI}
                          />
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>参考列表</Label>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() =>
                        patch({ references: [...value.references, createEmptyReferenceWork()] })
                      }
                      disabled={lockUI}
                    >
                      <Plus className="h-3 w-3 mr-1" />
                      添加参考
                    </Button>
                  </div>
                  {value.references.length === 0 ? (
                    <p className="text-xs text-muted-foreground">暂未填写参考作品</p>
                  ) : (
                    <div className="space-y-2">
                      {value.references.map((reference, index) => (
                        <div key={reference.id} className="rounded-md border p-2 space-y-2">
                          <div className="flex items-center justify-between">
                            <span className="text-xs text-muted-foreground">{index + 1}.</span>
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              className="h-6 w-6 text-destructive"
                              onClick={() =>
                                patch({ references: removeById(value.references, reference.id) })
                              }
                              disabled={lockUI}
                            >
                              <Trash2 className="h-3 w-3" />
                            </Button>
                          </div>
                          <Input
                            placeholder="作品名"
                            value={reference.title}
                            onChange={(event) =>
                              patch({
                                references: updateById(value.references, reference.id, (item) => ({
                                  ...item,
                                  title: event.target.value,
                                })),
                              })
                            }
                            disabled={lockUI}
                          />
                          <Textarea
                            rows={2}
                            placeholder="借鉴点"
                            value={reference.inspiration}
                            onChange={(event) =>
                              patch({
                                references: updateById(value.references, reference.id, (item) => ({
                                  ...item,
                                  inspiration: event.target.value,
                                })),
                              })
                            }
                            disabled={lockUI}
                          />
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </>
            ) : null}

            <div className="flex items-center justify-between border-t pt-3 mt-1">
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  void tryNavigateTo(stepIndex - 1);
                }}
                disabled={lockUI || stepIndex === 0}
              >
                上一步
              </Button>
              <Button
                type="button"
                onClick={() => {
                  void tryNavigateTo(stepIndex + 1);
                }}
                disabled={lockUI || stepIndex === WIZARD_STEPS.length - 1}
              >
                下一步
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default NovelSettingsWizard;
