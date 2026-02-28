/**
 * 小说创作设定强类型定义（V1）
 *
 * 参考: /Users/coso/Documents/dev/js/ainovel/src/components/novel-settings/types.ts
 */

export const NOVEL_SETTINGS_SCHEMA_VERSION = 1;

const randomId = () => Math.random().toString(36).slice(2) + Date.now().toString(36);

export const ALL_NOVEL_GENRES = [
  "玄幻",
  "仙侠",
  "奇幻",
  "武侠",
  "都市",
  "言情",
  "科幻",
  "系统文",
  "后宫",
  "无限流",
  "悬疑",
  "历史",
  "军事",
  "游戏",
  "末世",
  "穿越",
  "重生",
  "网游",
  "竞技",
  "机甲",
  "星际",
  "灵异",
  "恐怖",
  "宫斗",
  "种田",
  "娱乐圈",
  "盗墓",
  "洪荒",
] as const;

export type NovelGenre = (typeof ALL_NOVEL_GENRES)[number];

export interface MainCharacter {
  name: string;
  gender: string;
  age: string;
  personality: string;
}

export interface SideCharacter {
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

export interface Antagonist extends SideCharacter {
  motive: string;
  fate: string;
}

export interface WorldDetails {
  powerSystem: string;
  factions: string;
  historyEvents: string;
  importantLocations: string;
  cultureAndTaboos: string;
}

export interface PlotBeat {
  id: string;
  title: string;
  detail: string;
}

export interface WritingStyle {
  narration: string;
  tones: string[];
  cheatLevel: string;
  focusAreas: string[];
  wordsPerChapter: number;
  temperature: number;
}

export interface TabooRule {
  id: string;
  content: string;
}

export interface ReferenceWork {
  id: string;
  title: string;
  inspiration: string;
}

export interface NovelSettingsV1 {
  genres: NovelGenre[];
  oneLinePitch: string;
  mainCharacter: MainCharacter;
  sideCharacters: SideCharacter[];
  antagonists: Antagonist[];
  worldSummary: string;
  conflictTheme: string;
  worldDetails: WorldDetails;
  opening: string;
  middleBeats: PlotBeat[];
  endingType: string;
  subplots: PlotBeat[];
  writingStyle: WritingStyle;
  totalWords: number;
  chapterWords: number;
  nsfw: boolean;
  systemNovel: boolean;
  harem: boolean;
  taboos: TabooRule[];
  references: ReferenceWork[];
}

export interface NovelSettingsEnvelope {
  schema_version: number;
  data: NovelSettingsV1;
}

export const createEmptySideCharacter = (): SideCharacter => ({
  id: randomId(),
  name: "",
  nickname: "",
  gender: "男",
  age: "",
  relationship: "",
  relationshipCustom: "",
  personalityTags: [],
  background: "",
  abilities: "",
  role: "",
  arc: "",
  arcCustom: "",
});

export const createEmptyAntagonist = (): Antagonist => ({
  ...createEmptySideCharacter(),
  motive: "",
  fate: "",
});

export const createEmptyPlotBeat = (): PlotBeat => ({
  id: randomId(),
  title: "",
  detail: "",
});

export const createEmptyTaboo = (): TabooRule => ({
  id: randomId(),
  content: "",
});

export const createEmptyReferenceWork = (): ReferenceWork => ({
  id: randomId(),
  title: "",
  inspiration: "",
});

export const createDefaultNovelSettingsV1 = (): NovelSettingsV1 => ({
  genres: [],
  oneLinePitch: "",
  mainCharacter: {
    name: "",
    gender: "男",
    age: "",
    personality: "",
  },
  sideCharacters: [],
  antagonists: [],
  worldSummary: "",
  conflictTheme: "",
  worldDetails: {
    powerSystem: "",
    factions: "",
    historyEvents: "",
    importantLocations: "",
    cultureAndTaboos: "",
  },
  opening: "",
  middleBeats: [],
  endingType: "",
  subplots: [],
  writingStyle: {
    narration: "第三人称有限",
    tones: [],
    cheatLevel: "稳步成长",
    focusAreas: [],
    wordsPerChapter: 3000,
    temperature: 0.7,
  },
  totalWords: 100000,
  chapterWords: 3000,
  nsfw: false,
  systemNovel: false,
  harem: false,
  taboos: [],
  references: [],
});

export const createDefaultNovelSettingsEnvelope = (): NovelSettingsEnvelope => ({
  schema_version: NOVEL_SETTINGS_SCHEMA_VERSION,
  data: createDefaultNovelSettingsV1(),
});

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asString(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function asBoolean(value: unknown, fallback = false): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function asNumber(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((item): item is string => typeof item === "string");
}

function normalizeMainCharacter(value: unknown): MainCharacter {
  const base = createDefaultNovelSettingsV1().mainCharacter;
  if (!isRecord(value)) {
    return base;
  }
  return {
    name: asString(value.name, base.name),
    gender: asString(value.gender, base.gender),
    age: asString(value.age, base.age),
    personality: asString(value.personality, base.personality),
  };
}

function normalizeSideCharacter(value: unknown): SideCharacter {
  const base = createEmptySideCharacter();
  if (!isRecord(value)) {
    return base;
  }
  return {
    id: asString(value.id, base.id),
    name: asString(value.name, base.name),
    nickname: asString(value.nickname, base.nickname),
    gender: asString(value.gender, base.gender),
    age: asString(value.age, base.age),
    relationship: asString(value.relationship, base.relationship),
    relationshipCustom: asString(value.relationshipCustom, base.relationshipCustom),
    personalityTags: asStringArray(value.personalityTags),
    background: asString(value.background, base.background),
    abilities: asString(value.abilities, base.abilities),
    role: asString(value.role, base.role),
    arc: asString(value.arc, base.arc),
    arcCustom: asString(value.arcCustom, base.arcCustom),
  };
}

function normalizeAntagonist(value: unknown): Antagonist {
  const base = createEmptyAntagonist();
  if (!isRecord(value)) {
    return base;
  }
  const side = normalizeSideCharacter(value);
  return {
    ...side,
    motive: asString(value.motive, base.motive),
    fate: asString(value.fate, base.fate),
  };
}

function normalizeWorldDetails(value: unknown): WorldDetails {
  const base = createDefaultNovelSettingsV1().worldDetails;
  if (!isRecord(value)) {
    return base;
  }
  return {
    powerSystem: asString(value.powerSystem, base.powerSystem),
    factions: asString(value.factions, base.factions),
    historyEvents: asString(value.historyEvents, base.historyEvents),
    importantLocations: asString(value.importantLocations, base.importantLocations),
    cultureAndTaboos: asString(value.cultureAndTaboos, base.cultureAndTaboos),
  };
}

function normalizePlotBeat(value: unknown): PlotBeat {
  const base = createEmptyPlotBeat();
  if (!isRecord(value)) {
    return base;
  }
  return {
    id: asString(value.id, base.id),
    title: asString(value.title, base.title),
    detail: asString(value.detail, base.detail),
  };
}

function normalizeWritingStyle(value: unknown): WritingStyle {
  const base = createDefaultNovelSettingsV1().writingStyle;
  if (!isRecord(value)) {
    return base;
  }
  return {
    narration: asString(value.narration, base.narration),
    tones: asStringArray(value.tones),
    cheatLevel: asString(value.cheatLevel, base.cheatLevel),
    focusAreas: asStringArray(value.focusAreas),
    wordsPerChapter: asNumber(value.wordsPerChapter, base.wordsPerChapter),
    temperature: asNumber(value.temperature, base.temperature),
  };
}

function normalizeTaboo(value: unknown): TabooRule {
  const base = createEmptyTaboo();
  if (typeof value === "string") {
    return { ...base, content: value };
  }
  if (!isRecord(value)) {
    return base;
  }
  return {
    id: asString(value.id, base.id),
    content: asString(value.content, base.content),
  };
}

function normalizeReference(value: unknown): ReferenceWork {
  const base = createEmptyReferenceWork();
  if (!isRecord(value)) {
    return base;
  }
  return {
    id: asString(value.id, base.id),
    title: asString(value.title, base.title),
    inspiration: asString(value.inspiration, base.inspiration),
  };
}

export function normalizeNovelSettings(value: unknown): NovelSettingsV1 {
  const base = createDefaultNovelSettingsV1();
  if (!isRecord(value)) {
    return base;
  }

  const genres = Array.isArray(value.genres)
    ? value.genres.filter((item): item is NovelGenre => typeof item === "string")
    : base.genres;

  return {
    genres,
    oneLinePitch: asString(value.oneLinePitch, base.oneLinePitch),
    mainCharacter: normalizeMainCharacter(value.mainCharacter),
    sideCharacters: Array.isArray(value.sideCharacters)
      ? value.sideCharacters.map(normalizeSideCharacter)
      : base.sideCharacters,
    antagonists: Array.isArray(value.antagonists)
      ? value.antagonists.map(normalizeAntagonist)
      : base.antagonists,
    worldSummary: asString(value.worldSummary, base.worldSummary),
    conflictTheme: asString(value.conflictTheme, base.conflictTheme),
    worldDetails: normalizeWorldDetails(value.worldDetails),
    opening: asString(value.opening, base.opening),
    middleBeats: Array.isArray(value.middleBeats)
      ? value.middleBeats.map(normalizePlotBeat)
      : base.middleBeats,
    endingType: asString(value.endingType, base.endingType),
    subplots: Array.isArray(value.subplots)
      ? value.subplots.map(normalizePlotBeat)
      : base.subplots,
    writingStyle: normalizeWritingStyle(value.writingStyle),
    totalWords: asNumber(value.totalWords, base.totalWords),
    chapterWords: asNumber(value.chapterWords, base.chapterWords),
    nsfw: asBoolean(value.nsfw, base.nsfw),
    systemNovel: asBoolean(value.systemNovel, base.systemNovel),
    harem: asBoolean(value.harem, base.harem),
    taboos: Array.isArray(value.taboos)
      ? value.taboos.map(normalizeTaboo).filter((item) => item.content.trim())
      : base.taboos,
    references: Array.isArray(value.references)
      ? value.references
          .map(normalizeReference)
          .filter((item) => item.title.trim() || item.inspiration.trim())
      : base.references,
  };
}

export function normalizeNovelSettingsEnvelope(value: unknown): NovelSettingsEnvelope {
  if (isRecord(value) && isRecord(value.data)) {
    const schemaVersion = asNumber(value.schema_version, NOVEL_SETTINGS_SCHEMA_VERSION);
    return {
      schema_version: schemaVersion,
      data: normalizeNovelSettings(value.data),
    };
  }

  return {
    schema_version: NOVEL_SETTINGS_SCHEMA_VERSION,
    data: normalizeNovelSettings(value),
  };
}
