/**
 * @file 音乐画布类型定义
 * @description 定义音乐画布相关的核心类型
 * @module components/content-creator/canvas/music/types
 */

// ============================================================
// 基础枚举类型
// ============================================================

/** 歌曲类型 */
export type SongType =
  | "pop" // 流行
  | "folk" // 民谣
  | "rock" // 摇滚
  | "guofeng" // 古风
  | "rap" // 说唱
  | "rnb" // R&B
  | "electronic" // 电子
  | "other"; // 其他

/** 创作模式 */
export type MusicCreationMode =
  | "coach" // 教练模式 - AI引导逐段创作
  | "express" // 快速模式 - AI直接生成
  | "hybrid"; // 混合模式 - AI生成框架，用户填充

/** 视图模式 */
export type MusicViewMode =
  | "lyrics" // 纯歌词
  | "numbered" // 简谱
  | "guitar" // 吉他谱
  | "piano"; // 钢琴谱

/** 段落类型 */
export type SectionType =
  | "intro" // 前奏
  | "verse" // 主歌
  | "pre-chorus" // 预副歌
  | "chorus" // 副歌
  | "bridge" // 桥段
  | "interlude" // 间奏
  | "outro"; // 尾奏

/** 情感基调 */
export type MoodType =
  | "joyful" // 欢快明亮
  | "gentle" // 温柔抒情
  | "sorrowful" // 忧伤低沉
  | "passionate" // 激昂热血
  | "mysterious" // 神秘空灵
  | "nostalgic"; // 怀旧复古

/** 装饰音类型 */
export type OrnamentType =
  | "slide" // 滑音
  | "trill" // 颤音
  | "grace" // 倚音
  | "mordent"; // 波音

// ============================================================
// 调式相关类型
// ============================================================

/** 西方调式 */
export type WesternMode =
  | "major" // 大调
  | "minor" // 小调
  | "dorian" // 多利亚
  | "mixolydian" // 混合利底亚
  | "blues"; // 蓝调

/** 中国五声调式 */
export type ChinesePentatonicMode =
  | "gong" // 宫调式
  | "shang" // 商调式
  | "jue" // 角调式
  | "zhi" // 徵调式
  | "yu"; // 羽调式

/** 调式类型 */
export type ModeType = WesternMode | ChinesePentatonicMode;

/** 调式检测结果 */
export interface ModeDetection {
  /** 检测到的调式 */
  detected: ModeType;
  /** 置信度 (0-100) */
  confidence: number;
  /** 情感特征描述 */
  emotionalFeature: string;
  /** 主音 */
  tonic: string;
}

// ============================================================
// 音乐元素类型
// ============================================================

/** 音符 */
export interface Note {
  /** 简谱音高 1-7，0 表示休止 */
  pitch: number;
  /** 八度，0=中音，1=高音，-1=低音 */
  octave: number;
  /** 时值，1=四分音符，0.5=八分音符，2=二分音符 */
  duration: number;
  /** 装饰音 */
  ornament?: OrnamentType;
  /** 是否为附点 */
  dotted?: boolean;
  /** 是否为连音 */
  tied?: boolean;
}

/** 小节 */
export interface Bar {
  /** 小节 ID */
  id: string;
  /** 小节序号 */
  barNumber: number;
  /** 和弦 */
  chord: string;
  /** 音符列表 */
  notes: Note[];
  /** 对应歌词 */
  lyrics: string;
}

/** 段落 */
export interface MusicSection {
  /** 段落 ID */
  id: string;
  /** 段落类型 */
  type: SectionType;
  /** 段落名称 */
  name: string;
  /** 排序序号 */
  order: number;
  /** 小节列表 */
  bars: Bar[];
  /** 歌词行 */
  lyricsLines: string[];
  /** 重复次数 */
  repeatCount?: number;
}

/** 和弦信息 */
export interface ChordInfo {
  /** 和弦名称 */
  name: string;
  /** 组成音 */
  notes: string[];
  /** 吉他指法 (6弦，-1表示不弹) */
  fingering: number[];
  /** 品位 */
  fret: number;
  /** 和弦图 SVG */
  diagram?: string;
}

// ============================================================
// 歌曲规格类型
// ============================================================

/** 歌曲规格 */
export interface SongSpec {
  /** 歌曲名称 */
  title: string;
  /** 歌曲类型 */
  songType: SongType;
  /** 歌曲主题描述 */
  theme: string;
  /** 调式 */
  key: string;
  /** 拍号 */
  timeSignature: string;
  /** 速度 (BPM) */
  tempo: number;
  /** 情感基调 */
  mood: MoodType;
  /** 创作模式 */
  creationMode: MusicCreationMode;
  /** 作词人 */
  lyricist?: string;
  /** 作曲人 */
  composer?: string;
}

// ============================================================
// 旋律学习相关类型
// ============================================================

/** 借鉴程度 */
export type BorrowLevel =
  | "light" // 轻度参考 - 仅借鉴调式和情绪
  | "style" // 风格借鉴 - 参考整体风格
  | "high"; // 高度相似 - 严格遵循特征

/** 旋律学习创作模式 */
export type MimicCreationMode =
  | "quick" // 快速模式 (3-8分钟)
  | "professional" // 专业模式 (10-18分钟)
  | "coach"; // 教练模式 (20-35分钟)

/** 节奏型统计 */
export interface RhythmStats {
  /** 四分音符占比 */
  quarterNote: number;
  /** 八分音符占比 */
  eighthNote: number;
  /** 附点节奏占比 */
  dottedNote: number;
  /** 切分节奏占比 */
  syncopation: number;
  /** 三连音占比 */
  triplet: number;
  /** 十六分音符占比 */
  sixteenth: number;
}

/** 音程统计 */
export interface IntervalStats {
  /** 同度占比 */
  unison: number;
  /** 级进占比 (2度) */
  stepwise: number;
  /** 小跳占比 (3-4度) */
  smallLeap: number;
  /** 大跳占比 (5度以上) */
  largeLeap: number;
}

/** 旋律轮廓统计 */
export interface ContourStats {
  /** 上行占比 */
  ascending: number;
  /** 下行占比 */
  descending: number;
  /** 平稳占比 */
  stable: number;
}

/** 旋律特征分析报告 */
export interface MelodyAnalysisReport {
  /** 总音符数 */
  totalNotes: number;
  /** 音域范围 */
  pitchRange: {
    min: number;
    max: number;
    minNote: string;
    maxNote: string;
  };
  /** Ticks per beat */
  ticksPerBeat: number;
  /** 调式检测 */
  mode: ModeDetection;
  /** 节奏型统计 */
  rhythmStats: RhythmStats;
  /** 音程统计 */
  intervalStats: IntervalStats;
  /** 旋律轮廓统计 */
  contourStats: ContourStats;
  /** 常用音程 */
  commonIntervals: { interval: string; count: number }[];
  /** 节奏密度 (音符/小节) */
  rhythmDensity: number;
}

/** 歌词段落分析 */
export interface LyricsSectionAnalysis {
  /** 段落类型 */
  type: SectionType;
  /** 段落名称 */
  name: string;
  /** 歌词行 */
  lines: string[];
  /** 字数 */
  charCount: number;
}

/** 意象分析 */
export interface ImageryAnalysis {
  /** 意象类别 */
  category: string;
  /** 具体意象 */
  items: string[];
  /** 效果描述 */
  effect: string;
}

/** 歌词深度分析报告 */
export interface LyricsAnalysisReport {
  /** 歌曲标题 */
  title: string;
  /** 艺术家 */
  artist?: string;
  /** 总行数 */
  totalLines: number;
  /** 总字数 */
  totalChars: number;
  /** 结构分析 */
  structure: LyricsSectionAnalysis[];
  /** 意象分析 */
  imagery: ImageryAnalysis[];
  /** 修辞手法 */
  rhetoric: string[];
  /** 风格特征 */
  styleFeatures: string[];
  /** 情感走向 */
  emotionalProgression: string;
  /** 押韵分析 */
  rhymeAnalysis: {
    scheme: string;
    quality: "strict" | "moderate" | "free";
  };
}

/** 一致性评分 */
export interface ConsistencyScore {
  /** 结构一致性 (0-100) */
  structure: number;
  /** 风格一致性 (0-100) */
  style: number;
  /** 旋律适配度 (0-100) */
  melodyFit: number;
  /** 综合评分 (0-100) */
  overall: number;
  /** 评分说明 */
  comments: string[];
}

/** 音轨候选 */
export interface TrackCandidate {
  /** 音轨索引 */
  index: number;
  /** 音轨名称 */
  name: string;
  /** 音符数量 */
  noteCount: number;
  /** 匹配分数 */
  score: number;
  /** 置信度 */
  confidence: "high" | "medium" | "low";
}

/** 音轨匹配结果 */
export interface TrackMatchResult {
  /** 选中的音轨 */
  selected: TrackCandidate | null;
  /** 所有候选音轨 */
  candidates: TrackCandidate[];
}

/** 旋律学习状态 */
export interface MelodyMimicState {
  /** 参考文件路径 */
  referenceFile: string | null;
  /** 参考文件类型 */
  referenceFileType: "midi" | "mp3" | null;
  /** 歌词文件路径 */
  lyricsFile: string | null;
  /** 歌词内容 */
  lyricsContent: string;
  /** 音轨匹配结果 */
  trackMatch: TrackMatchResult | null;
  /** 旋律分析报告 */
  melodyReport: MelodyAnalysisReport | null;
  /** 歌词分析报告 */
  lyricsReport: LyricsAnalysisReport | null;
  /** 创作模式 */
  creationMode: MimicCreationMode;
  /** 借鉴程度 */
  borrowLevel: BorrowLevel;
  /** 一致性评分 */
  consistencyScore: ConsistencyScore | null;
  /** 分析进度 (0-100) */
  analysisProgress: number;
  /** 分析状态 */
  analysisStatus: "idle" | "analyzing" | "completed" | "error";
  /** 错误信息 */
  errorMessage: string | null;
}

// ============================================================
// 画布状态类型
// ============================================================

/** 音乐版本 */
export interface MusicVersion {
  /** 版本 ID */
  id: string;
  /** 版本描述 */
  description: string;
  /** 创建时间 */
  createdAt: number;
  /** 歌曲规格快照 */
  spec: SongSpec;
  /** 段落快照 */
  sections: MusicSection[];
}

/** 音乐画布状态 */
export interface MusicCanvasState {
  /** 画布类型标识 */
  type: "music";
  /** 项目 ID */
  projectId: string;
  /** 歌曲规格 */
  spec: SongSpec;
  /** 段落列表 */
  sections: MusicSection[];
  /** 当前选中的段落 ID */
  currentSectionId: string | null;
  /** 当前选中的小节 ID */
  currentBarId: string | null;
  /** 视图模式 */
  viewMode: MusicViewMode;
  /** 是否正在播放 */
  isPlaying: boolean;
  /** 播放位置 (秒) */
  playbackPosition: number;
  /** 播放速度倍率 */
  playbackRate: number;
  /** 音量 (0-1) */
  volume: number;
  /** 是否循环播放 */
  isLooping: boolean;
  /** 版本历史 */
  versions: MusicVersion[];
  /** 当前版本 ID */
  currentVersionId: string;
  /** 是否处于编辑模式 */
  isEditing: boolean;
  /** 旋律学习状态 */
  mimicState: MelodyMimicState | null;
  /** 当前工作流类型 */
  workflowType: "original" | "mimic";
  /** 当前工作流步骤 */
  currentWorkflowStep: string;
}

// ============================================================
// 组件 Props 类型
// ============================================================

/** 音乐画布 Props */
export interface MusicCanvasProps {
  /** 画布状态 */
  state: MusicCanvasState;
  /** 状态变更回调 */
  onStateChange: (state: MusicCanvasState) => void;
  /** 返回首页回调 */
  onBackHome?: () => void;
  /** 关闭画布回调 */
  onClose: () => void;
  /** 是否正在流式输出 */
  isStreaming?: boolean;
}

/** 音乐工具栏 Props */
export interface MusicToolbarProps {
  /** 歌曲规格 */
  spec: SongSpec;
  /** 视图模式 */
  viewMode: MusicViewMode;
  /** 是否正在播放 */
  isPlaying: boolean;
  /** 是否可以撤销 */
  canUndo: boolean;
  /** 是否可以重做 */
  canRedo: boolean;
  /** 视图模式切换回调 */
  onViewModeChange: (mode: MusicViewMode) => void;
  /** 播放/暂停回调 */
  onPlayToggle: () => void;
  /** 撤销回调 */
  onUndo: () => void;
  /** 重做回调 */
  onRedo: () => void;
  /** 导出回调 */
  onExport: () => void;
  /** 关闭回调 */
  onClose: () => void;
}

/** 歌词编辑器 Props */
export interface LyricsEditorProps {
  /** 段落列表 */
  sections: MusicSection[];
  /** 当前段落 ID */
  currentSectionId: string | null;
  /** 段落变更回调 */
  onSectionsChange: (sections: MusicSection[]) => void;
  /** 段落选择回调 */
  onSectionSelect: (sectionId: string) => void;
}

/** 简谱编辑器 Props */
export interface JianpuEditorProps {
  /** 当前段落 */
  section: MusicSection | null;
  /** 当前小节 ID */
  currentBarId: string | null;
  /** 段落变更回调 */
  onSectionChange: (section: MusicSection) => void;
  /** 小节选择回调 */
  onBarSelect: (barId: string) => void;
}

// ============================================================
// 导出相关类型
// ============================================================

/** 导出格式 */
export type MusicExportFormat =
  | "pdf" // PDF 歌词本
  | "midi" // MIDI 文件
  | "musicxml" // MusicXML
  | "txt" // 纯文本歌词
  | "suno" // Suno 提示词
  | "tunee"; // Tunee 素材包

/** 导出配置 */
export interface MusicExportConfig {
  /** 导出格式 */
  format: MusicExportFormat;
  /** PDF 视图类型 */
  pdfView?: "lyrics" | "numbered" | "guitar" | "full";
  /** 是否包含和弦 */
  includeChords?: boolean;
  /** 是否包含简谱 */
  includeJianpu?: boolean;
  /** Suno 风格标签 */
  sunoTags?: string[];
}

/** Suno 导出结果 */
export interface SunoExport {
  /** 带标记的歌词 */
  lyrics: string;
  /** 风格提示词 */
  stylePrompt: string;
  /** 风格标签 */
  tags: string[];
}

/** Tunee 导出结果 */
export interface TuneeExport {
  /** 歌词文件内容 */
  lyrics: string;
  /** 对话素材 */
  dialogMaterial: string;
  /** 参考信息 */
  reference: string;
}

// ============================================================
// 工具函数
// ============================================================

/** 创建初始歌曲规格 */
export function createInitialSpec(): SongSpec {
  return {
    title: "未命名歌曲",
    songType: "pop",
    theme: "",
    key: "C",
    timeSignature: "4/4",
    tempo: 72,
    mood: "gentle",
    creationMode: "coach",
  };
}

/** 创建初始音乐画布状态 */
export function createInitialMusicState(): MusicCanvasState {
  const initialSpec = createInitialSpec();
  const initialVersion: MusicVersion = {
    id: crypto.randomUUID(),
    description: "初始版本",
    createdAt: Date.now(),
    spec: initialSpec,
    sections: [],
  };

  return {
    type: "music",
    projectId: crypto.randomUUID(),
    spec: initialSpec,
    sections: [],
    currentSectionId: null,
    currentBarId: null,
    viewMode: "lyrics",
    isPlaying: false,
    playbackPosition: 0,
    playbackRate: 1,
    volume: 0.8,
    isLooping: false,
    versions: [initialVersion],
    currentVersionId: initialVersion.id,
    isEditing: false,
    mimicState: null,
    workflowType: "original",
    currentWorkflowStep: "clarify",
  };
}

/** 创建初始旋律学习状态 */
export function createInitialMimicState(): MelodyMimicState {
  return {
    referenceFile: null,
    referenceFileType: null,
    lyricsFile: null,
    lyricsContent: "",
    trackMatch: null,
    melodyReport: null,
    lyricsReport: null,
    creationMode: "professional",
    borrowLevel: "style",
    consistencyScore: null,
    analysisProgress: 0,
    analysisStatus: "idle",
    errorMessage: null,
  };
}

/** 创建新段落 */
export function createSection(type: SectionType, order: number): MusicSection {
  const typeNames: Record<SectionType, string> = {
    intro: "前奏",
    verse: "主歌",
    "pre-chorus": "预副歌",
    chorus: "副歌",
    bridge: "桥段",
    interlude: "间奏",
    outro: "尾奏",
  };

  return {
    id: crypto.randomUUID(),
    type,
    name: typeNames[type],
    order,
    bars: [],
    lyricsLines: [],
  };
}

// ============================================================
// 常量映射
// ============================================================

/** 段落类型到 Suno 标签的映射 */
export const SECTION_TO_SUNO_TAG: Record<SectionType, string> = {
  intro: "Intro",
  verse: "Verse",
  "pre-chorus": "Pre-Chorus",
  chorus: "Chorus",
  bridge: "Bridge",
  interlude: "Interlude",
  outro: "Outro",
};

/** 调式情感特征映射 */
export const MODE_EMOTIONAL_FEATURES: Record<ModeType, string> = {
  major: "明亮、欢快、积极",
  minor: "忧伤、深沉、内敛",
  dorian: "爵士、蓝调、神秘",
  mixolydian: "摇滚、乡村、放松",
  blues: "蓝调、忧郁、深情",
  gong: "明亮、庄重、喜庆",
  shang: "深沉、内敛、怀旧",
  jue: "清新、空灵、禅意",
  zhi: "热情、奔放、豪迈",
  yu: "忧伤、婉转、抒情",
};

/** 歌曲类型显示名称 */
export const SONG_TYPE_LABELS: Record<SongType, string> = {
  pop: "流行",
  folk: "民谣",
  rock: "摇滚",
  guofeng: "古风",
  rap: "说唱",
  rnb: "R&B",
  electronic: "电子",
  other: "其他",
};

/** 情感基调显示名称 */
export const MOOD_LABELS: Record<MoodType, string> = {
  joyful: "欢快明亮",
  gentle: "温柔抒情",
  sorrowful: "忧伤低沉",
  passionate: "激昂热血",
  mysterious: "神秘空灵",
  nostalgic: "怀旧复古",
};

/** 创作模式显示名称 */
export const CREATION_MODE_LABELS: Record<MusicCreationMode, string> = {
  coach: "教练模式",
  express: "快速模式",
  hybrid: "混合模式",
};

/** 视图模式显示名称 */
export const VIEW_MODE_LABELS: Record<MusicViewMode, string> = {
  lyrics: "歌词",
  numbered: "简谱",
  guitar: "吉他谱",
  piano: "钢琴谱",
};
