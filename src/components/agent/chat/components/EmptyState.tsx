import React, { useState, useEffect } from "react";
import styled, { keyframes, css } from "styled-components";
import {
  ArrowRight,
  ImageIcon,
  Video,
  FileText,
  PenTool,
  BrainCircuit,
  CalendarRange,
  ChevronDown,
  Search,
  Globe,
  Music,
} from "lucide-react";
import { getConfig } from "@/hooks/useTauri";
import type { CreationMode } from "./types";
import { CREATION_MODE_CONFIG } from "./constants";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { ProjectSelector } from "@/components/projects/ProjectSelector";
import { useProjects } from "@/hooks/useProjects";
import { Badge } from "@/components/ui/badge";

// Import Assets
import iconXhs from "@/assets/platforms/xhs.png";
import iconGzh from "@/assets/platforms/gzh.png";
import iconZhihu from "@/assets/platforms/zhihu.png";
import iconToutiao from "@/assets/platforms/toutiao.png";
import iconJuejin from "@/assets/platforms/juejin.png";
import iconCsdn from "@/assets/platforms/csdn.png";

// --- Animations ---
const fadeIn = keyframes`
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
`;

// --- Styled Components ---

const Container = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 40px 20px;
  background-color: hsl(var(--background));
  overflow-y: auto;
  position: relative;

  // Subtle mesh background effect
  &::before {
    content: "";
    position: absolute;
    top: -10%;
    left: 20%;
    width: 600px;
    height: 600px;
    background: radial-gradient(
      circle,
      hsl(var(--primary) / 0.05) 0%,
      transparent 70%
    );
    border-radius: 50%;
    pointer-events: none;
    z-index: 0;
  }
`;

const ContentWrapper = styled.div`
  max-width: 900px;
  width: 100%;
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 36px;
  animation: ${fadeIn} 0.5s ease-out;
`;

const Header = styled.div`
  text-align: center;
  margin-bottom: 8px;
`;

const shimmer = keyframes`
  0% { background-position: 0% 50%; filter: brightness(100%); }
  50% { background-position: 100% 50%; filter: brightness(120%); }
  100% { background-position: 0% 50%; filter: brightness(100%); }
`;

const MainTitle = styled.h1`
  font-size: 42px;
  font-weight: 800;
  color: hsl(var(--foreground));
  margin-bottom: 16px;
  letter-spacing: -1px;
  line-height: 1.15;

  // Advanced Light & Shadow Gradient
  background: linear-gradient(
    135deg,
    hsl(var(--foreground)) 0%,
    #8b5cf6 25%,
    #ec4899 50%,
    #8b5cf6 75%,
    hsl(var(--foreground)) 100%
  );
  background-size: 300% auto;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;

  // Animation
  animation: ${shimmer} 5s ease-in-out infinite;

  // Optical Glow
  filter: drop-shadow(0 0 20px rgba(139, 92, 246, 0.3));

  span {
    display: block; // Force new line for the second part naturally if needed, or keep inline
    background: linear-gradient(to right, #6366f1, #a855f7, #ec4899);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }
`;

// --- Custom Tabs ---
const TabsContainer = styled.div`
  display: flex;
  gap: 8px;
  padding: 6px;
  background-color: hsl(var(--muted) / 0.4);
  backdrop-filter: blur(10px);
  border-radius: 16px;
  border: 1px solid hsl(var(--border) / 0.5);
  box-shadow:
    0 4px 6px -1px rgba(0, 0, 0, 0.01),
    0 2px 4px -1px rgba(0, 0, 0, 0.01);
  overflow-x: auto;
  max-width: 100%;
  scrollbar-width: none; // hide scrollbar
`;

const TabItem = styled.button<{ $active?: boolean }>`
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.25s cubic-bezier(0.25, 1, 0.5, 1);
  white-space: nowrap;

  ${(props) =>
    props.$active
      ? css`
          background-color: hsl(var(--background));
          color: hsl(var(--foreground));
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
          transform: scale(1.02);
        `
      : css`
          color: hsl(var(--muted-foreground));
          &:hover {
            background-color: hsl(var(--muted) / 0.5);
            color: hsl(var(--foreground));
          }
        `}
`;

// --- Input Card ---
const InputCard = styled.div`
  width: 100%;
  position: relative;
  background-color: hsl(var(--card));
  border: 1px solid hsl(var(--border) / 0.6);
  border-radius: 20px;
  box-shadow:
    0 20px 40px -5px rgba(0, 0, 0, 0.03),
    0 8px 16px -4px rgba(0, 0, 0, 0.03);
  overflow: visible; // Allow dropdowns to overflow
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);

  &:hover {
    box-shadow:
      0 25px 50px -12px rgba(0, 0, 0, 0.06),
      0 12px 24px -6px rgba(0, 0, 0, 0.04);
    border-color: hsl(var(--primary) / 0.3);
  }

  &:focus-within {
    border-color: hsl(var(--primary));
    box-shadow:
      0 0 0 4px hsl(var(--primary) / 0.1),
      0 25px 50px -12px rgba(0, 0, 0, 0.08);
  }
`;

const StyledTextarea = styled(Textarea)`
  min-height: 150px;
  padding: 24px 28px;
  border: none;
  font-size: 16px;
  line-height: 1.6;
  resize: none;
  background: transparent;
  color: hsl(var(--foreground));

  &::placeholder {
    color: hsl(var(--muted-foreground) / 0.7);
    font-weight: 300;
  }

  &:focus-visible {
    ring: 0;
    outline: none;
    box-shadow: none;
  }
`;

const Toolbar = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px 16px 20px;
  background: linear-gradient(to bottom, transparent, hsl(var(--muted) / 0.2));
  border-bottom-left-radius: 20px;
  border-bottom-right-radius: 20px;
`;

const ToolLoginLeft = styled.div`
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
`;

// --- Styles for Selectors ---
const ColorDot = styled.div<{ $color: string }>`
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background-color: ${(props) => props.$color};
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.1) inset;
`;

const GridSelect = styled.div`
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  padding: 8px;
`;

const GridItem = styled.div<{ $active?: boolean }>`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 10px;
  border-radius: 8px;
  border: 1px solid
    ${(props) => (props.$active ? "hsl(var(--primary))" : "transparent")};
  background-color: ${(props) =>
    props.$active ? "hsl(var(--primary)/0.08)" : "hsl(var(--muted)/0.3)"};
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    background-color: hsl(var(--primary) / 0.05);
  }
`;

interface EmptyStateProps {
  input: string;
  setInput: (value: string) => void;
  onSend: (value: string) => void;
  /** 创作模式 */
  creationMode?: CreationMode;
  /** 创作模式变更回调 */
  onCreationModeChange?: (mode: CreationMode) => void;
  /** 当前激活的主题 */
  activeTheme?: string;
  /** 主题变更回调 */
  onThemeChange?: (theme: string) => void;
  /** 推荐标签点击回调 */
  onRecommendationClick?: (shortLabel: string, fullPrompt: string) => void;
  /** 当前选中的项目 ID */
  projectId?: string | null;
  /** 项目变更回调 */
  onProjectChange?: (projectId: string) => void;
}

// Scenarios Configuration - 与 ProjectType 统一
const ALL_CATEGORIES = [
  {
    id: "general",
    label: "通用对话",
    icon: <Globe className="w-4 h-4" />,
  },
  {
    id: "social-media",
    label: "社媒内容",
    icon: <PenTool className="w-4 h-4" />,
  },
  { id: "poster", label: "图文海报", icon: <ImageIcon className="w-4 h-4" /> },
  { id: "music", label: "歌词曲谱", icon: <Music className="w-4 h-4" /> },
  {
    id: "knowledge",
    label: "知识探索",
    icon: <BrainCircuit className="w-4 h-4" />,
  },
  {
    id: "planning",
    label: "计划规划",
    icon: <CalendarRange className="w-4 h-4" />,
  },
  { id: "document", label: "办公文档", icon: <FileText className="w-4 h-4" /> },
  { id: "video", label: "短视频", icon: <Video className="w-4 h-4" /> },
  { id: "novel", label: "小说创作", icon: <PenTool className="w-4 h-4" /> },
];

/** 默认启用的主题 */
const DEFAULT_ENABLED_THEMES = [
  "general",
  "social-media",
  "poster",
  "music",
  "video",
  "novel",
];

// 需要显示创作模式选择器的主题
const CREATION_THEMES = [
  "social-media",
  "poster",
  "document",
  "video",
  "music",
  "novel",
];

/**
 * 推荐内容配置
 * 格式: [简化标题, 完整 Prompt]
 * 简化标题用于显示，完整 Prompt 用于点击发送
 */
const RECOMMENDATIONS: Record<string, [string, string][]> = {
  "social-media": [
    [
      "爆款标题生成",
      "帮我为'春季护肤routine'写10个小红书爆款标题，要求：数字开头、制造悬念、引发共鸣",
    ],
    [
      "小红书探店文案",
      "写一篇小红书探店文案：周末在杭州发现一家宝藏咖啡店，工业风装修+拉花拿铁，适合拍照出片",
    ],
    [
      "公众号排版",
      "帮我把这段话排版成公众号风格：每段不超过150字，加入小标题和emoji，重点内容加粗",
    ],
    [
      "评论区回复",
      "用户评论'这个产品真的好用吗？还是广告？'，帮我写一条真诚、有说服力的回复",
    ],
  ],
  poster: [
    [
      "海报设计",
      "设计一张夏日音乐节海报：主色调渐变蓝紫，中央是剪影吉他和声波元素，底部大标题'夏日音浪'",
    ],
    [
      "插画生成",
      "生成一幅温馨的卧室插画：暖色调，落地窗透进阳光，书桌上有绿植和笔记本，治愈系风格",
    ],
    [
      "UI 界面",
      "设计一个健身APP首页：深色模式，顶部显示今日步数，中间是环形进度条，底部四个功能入口",
    ],
    [
      "Logo 设计",
      "设计一家名为'绿野'的有机食品品牌Logo：简约绿色叶子轮廓，可单独使用，适合多种尺寸",
    ],
    [
      "摄影修图",
      "人像照片调色建议：肤色通透，背景偏暖，整体日系清新风格，降低对比度提升亮度",
    ],
  ],
  knowledge: [
    [
      "解释量子计算",
      "用通俗易懂的方式解释量子计算是什么，类比成生活中的例子，适合非理科背景的人理解",
    ],
    [
      "总结这篇论文",
      "[粘贴论文链接或内容后] 帮我总结这篇论文的核心观点、研究方法和主要结论，输出500字以内的摘要",
    ],
    [
      "如何制定OKR",
      "详细介绍OKR（目标与关键结果）制定方法，包括设定原则、常见误区和实际案例，适合团队管理者",
    ],
    [
      "分析行业趋势",
      "分析2024年AI行业发展趋势，从技术突破、商业化进程、监管政策三个维度展开",
    ],
  ],
  planning: [
    [
      "日本旅行计划",
      "帮我制定一个7天日本关西旅行计划：大阪进京都出，包含主要景点、美食推荐、交通路线和预算估算",
    ],
    [
      "年度职业规划",
      "制定一名前端开发工程师的2024年职业规划：技能提升、项目经验、人脉积累、求职目标四个维度",
    ],
    [
      "婚礼流程表",
      "制定一场户外草坪婚礼的流程表：上午10点开始，包含仪式、宴会、互动环节，标注每个环节的时间",
    ],
    [
      "健身计划",
      "为办公室上班族制定健身计划：每周3次，每次30分钟，无需器械，可在办公室或家中完成",
    ],
  ],
  music: [
    [
      "流行情歌",
      "创作一首关于'暗恋'的流行情歌：主歌描述图书馆偶遇，副歌表达不敢告白的纠结，温柔的R&B风格",
    ],
    [
      "古风歌词",
      "创作古风歌词：主题是'江湖离别'，意象包括酒、剑、残阳、孤舟，五言句式为主，押韵工整",
    ],
    [
      "说唱歌词",
      "创作一段励志说唱：主题是'逆风翻盘'，讲述从低谷到成功的经历，快节奏，押韵密集，副歌要炸",
    ],
    [
      "儿歌创作",
      "创作一首儿童安全教育儿歌：主题是'过马路要小心'，简单易记，欢快活泼，3-5岁儿童能跟着唱",
    ],
    [
      "旋律学习",
      "帮我分析《稻香》的旋律特点：调式、和弦进行、节奏型，以及为什么听起来很怀旧温暖",
    ],
  ],
  novel: [
    [
      "玄幻小说",
      "创作玄幻小说开篇：主角在深山古洞觉醒传承，获得上古剑诀，第一章包含世界观铺垫和悬念设置",
    ],
    [
      "都市言情",
      "创作都市言情小说开篇：职场新人与高冷上司因工作误会相识，第一章突出女主性格和两人的初次冲突",
    ],
    [
      "悬疑推理",
      "创作悬疑推理小说开篇：雨夜发生密室杀人案，侦探到达现场发现三条线索，第一章制造悬念和推理伏笔",
    ],
    [
      "科幻未来",
      "创作科幻小说开篇：2084年人类首次接触外星文明，主角作为语言学家被召唤，第一章描写接触场景和紧张氛围",
    ],
    [
      "历史架空",
      "创作历史架空小说开篇：三国时期，一个现代人穿越成普通士兵，如何利用现代知识在乱世中生存",
    ],
  ],
};

// 主题对应的图标
const THEME_ICONS: Record<string, string> = {
  "social-media": "✨",
  poster: "🎨",
  knowledge: "🔍",
  planning: "📅",
  music: "🎵",
  novel: "📖",
};

export const EmptyState: React.FC<EmptyStateProps> = ({
  input,
  setInput,
  onSend,
  creationMode = "guided",
  onCreationModeChange,
  activeTheme = "general",
  onThemeChange,
  onRecommendationClick,
  projectId: externalProjectId,
  onProjectChange,
}) => {
  // 项目管理 - 内部状态（当外部未提供时使用）
  const { defaultProject, getOrCreateDefault } = useProjects();
  const [internalProjectId, setInternalProjectId] = useState<string | null>(
    null,
  );

  // 使用外部或内部的 projectId
  const projectId = externalProjectId ?? internalProjectId;

  // 初始化默认项目
  useEffect(() => {
    if (!externalProjectId && !internalProjectId) {
      if (defaultProject) {
        // 通知父组件
        if (onProjectChange) {
          onProjectChange(defaultProject.id);
        } else {
          setInternalProjectId(defaultProject.id);
        }
      } else {
        getOrCreateDefault().then((p) => {
          // 通知父组件
          if (onProjectChange) {
            onProjectChange(p.id);
          } else {
            setInternalProjectId(p.id);
          }
        });
      }
    }
  }, [
    externalProjectId,
    internalProjectId,
    defaultProject,
    getOrCreateDefault,
    onProjectChange,
  ]);

  // 处理项目变更
  const handleProjectChange = (newProjectId: string) => {
    if (onProjectChange) {
      onProjectChange(newProjectId);
    } else {
      setInternalProjectId(newProjectId);
    }
  };

  // 从配置中读取启用的主题
  const [enabledThemes, setEnabledThemes] = useState<string[]>(
    DEFAULT_ENABLED_THEMES,
  );

  // 加载配置
  useEffect(() => {
    const loadEnabledThemes = async () => {
      try {
        const config = await getConfig();
        if (config.content_creator?.enabled_themes) {
          setEnabledThemes(config.content_creator.enabled_themes);
        }
      } catch (e) {
        console.error("加载主题配置失败:", e);
      }
    };
    loadEnabledThemes();

    // 监听主题配置变更事件
    const handleThemeConfigChange = () => {
      loadEnabledThemes();
    };
    window.addEventListener("theme-config-changed", handleThemeConfigChange);

    return () => {
      window.removeEventListener(
        "theme-config-changed",
        handleThemeConfigChange,
      );
    };
  }, []);

  // 过滤后的主题列表
  const categories = ALL_CATEGORIES.filter((cat) =>
    enabledThemes.includes(cat.id),
  );

  // 使用外部传入的 activeTheme，如果有 onThemeChange 则使用受控模式
  const handleThemeChange = (theme: string) => {
    if (onThemeChange) {
      onThemeChange(theme);
    }
  };

  // 判断当前主题是否需要显示创作模式选择器
  const showCreationModeSelector = CREATION_THEMES.includes(activeTheme);

  // Local state for parameters (Mocking visual state)
  const [platform, setPlatform] = useState("xiaohongshu");
  const [ratio, setRatio] = useState("3:4");
  const [style, setStyle] = useState("minimal");
  const [depth, setDepth] = useState("deep");
  // Popover 打开状态
  const [ratioPopoverOpen, setRatioPopoverOpen] = useState(false);
  const [stylePopoverOpen, setStylePopoverOpen] = useState(false);

  const handleSend = () => {
    if (!input.trim()) return;
    let prefix = "";
    if (activeTheme === "social-media") prefix = `[社媒创作: ${platform}] `;
    if (activeTheme === "poster") prefix = `[图文生成: ${ratio}, ${style}] `;
    if (activeTheme === "video") prefix = `[视频脚本] `;
    if (activeTheme === "document") prefix = `[办公文档] `;
    if (activeTheme === "music") prefix = `[歌词曲谱] `;
    if (activeTheme === "novel") prefix = `[小说创作] `;
    if (activeTheme === "knowledge")
      prefix = `[知识探索: ${depth === "deep" ? "深度" : "快速"}] `;
    if (activeTheme === "planning") prefix = `[计划规划] `;

    onSend(prefix + input);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // Dynamic Placeholder
  const getPlaceholder = () => {
    switch (activeTheme) {
      case "knowledge":
        return "想了解什么？我可以帮你深度搜索、解析概念或总结长文...";
      case "planning":
        return "告诉我你的目标，无论是旅行计划、职业规划还是活动筹备...";
      case "social-media":
        return "输入主题，帮你创作小红书爆款文案、公众号文章...";
      case "poster":
        return "描述画面主体、风格、构图，生成精美海报或插画...";
      case "video":
        return "输入视频主题，生成分镜脚本和口播文案...";
      case "document":
        return "输入需求，生成周报、汇报PPT大纲或商务邮件...";
      case "music":
        return "输入歌曲主题或情感，帮你创作歌词、设计旋律...";
      case "novel":
        return "输入小说主题或情节，帮你创作章节内容...";
      case "general":
        return "有什么我可以帮你的？";
      default:
        return "输入你的想法...";
    }
  };

  // Helper to get platform icon
  const getPlatformIcon = (val: string) => {
    if (val === "xiaohongshu") return iconXhs;
    if (val === "wechat") return iconGzh;
    if (val === "zhihu") return iconZhihu;
    if (val === "toutiao") return iconToutiao;
    if (val === "juejin") return iconJuejin;
    if (val === "csdn") return iconCsdn;
    return undefined;
  };

  // Helper to get platform label
  const getPlatformLabel = (val: string) => {
    if (val === "xiaohongshu") return "小红书";
    if (val === "wechat") return "公众号";
    if (val === "zhihu") return "知乎";
    if (val === "toutiao") return "头条";
    if (val === "juejin") return "掘金";
    if (val === "csdn") return "CSDN";
    return val;
  };

  return (
    <Container>
      <ContentWrapper>
        <Header>
          <MainTitle>
            你想在这个平台 <br />
            <span>完成什么？</span>
          </MainTitle>
        </Header>

        <TabsContainer>
          {categories.map((cat) => (
            <TabItem
              key={cat.id}
              $active={activeTheme === cat.id}
              onClick={() => handleThemeChange(cat.id)}
            >
              <span
                className={
                  activeTheme === cat.id ? "text-primary" : "opacity-70"
                }
              >
                {cat.icon}
              </span>
              {cat.label}
            </TabItem>
          ))}
        </TabsContainer>

        {/* 输入卡片 */}
        <InputCard>
          <StyledTextarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={getPlaceholder()}
          />

          <Toolbar>
            <ToolLoginLeft>
              {/* 项目选择器 - PRD 4.2：始终显示，默认选中「默认项目」，按主题筛选 */}
              <ProjectSelector
                value={projectId}
                onChange={handleProjectChange}
                workspaceType={activeTheme}
                placeholder="选择项目"
                className="h-8 text-xs min-w-[120px]"
              />

              {activeTheme === "social-media" && (
                <>
                  <Select
                    value={platform}
                    onValueChange={setPlatform}
                    closeOnMouseLeave
                  >
                    <SelectTrigger className="h-8 text-xs bg-background border shadow-sm min-w-[120px]">
                      <div className="flex items-center gap-2">
                        {getPlatformIcon(platform) && (
                          <img
                            src={getPlatformIcon(platform)}
                            className="w-4 h-4 rounded-full"
                          />
                        )}
                        <span>{getPlatformLabel(platform)}</span>
                      </div>
                    </SelectTrigger>
                    <SelectContent className="p-1" side="top">
                      <div className="px-2 py-1.5 text-xs text-muted-foreground font-medium">
                        选择要创作的内容平台
                      </div>
                      <SelectItem value="xiaohongshu">
                        <div className="flex items-center gap-2">
                          <img src={iconXhs} className="w-4 h-4 rounded-full" />{" "}
                          小红书
                        </div>
                      </SelectItem>
                      <SelectItem value="wechat">
                        <div className="flex items-center gap-2">
                          <img src={iconGzh} className="w-4 h-4 rounded-full" />{" "}
                          公众号
                        </div>
                      </SelectItem>
                      <SelectItem value="toutiao">
                        <div className="flex items-center gap-2">
                          <img
                            src={iconToutiao}
                            className="w-4 h-4 rounded-full"
                          />{" "}
                          今日头条
                        </div>
                      </SelectItem>
                      <SelectItem value="zhihu">
                        <div className="flex items-center gap-2">
                          <img
                            src={iconZhihu}
                            className="w-4 h-4 rounded-full"
                          />{" "}
                          知乎
                        </div>
                      </SelectItem>
                      <SelectItem value="juejin">
                        <div className="flex items-center gap-2">
                          <img
                            src={iconJuejin}
                            className="w-4 h-4 rounded-full"
                          />{" "}
                          掘金
                        </div>
                      </SelectItem>
                      <SelectItem value="csdn">
                        <div className="flex items-center gap-2">
                          <img
                            src={iconCsdn}
                            className="w-4 h-4 rounded-full"
                          />{" "}
                          CSDN
                        </div>
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </>
              )}

              {/* 创作模式选择器 - 针对内容创作类主题 */}
              {showCreationModeSelector && (
                <Select
                  value={creationMode}
                  onValueChange={(val) =>
                    onCreationModeChange?.(val as CreationMode)
                  }
                >
                  <SelectTrigger className="h-8 text-xs bg-background border shadow-sm min-w-[120px]">
                    <div className="flex items-center gap-2">
                      {CREATION_MODE_CONFIG[creationMode].icon}
                      <span>{CREATION_MODE_CONFIG[creationMode].name}</span>
                    </div>
                  </SelectTrigger>
                  <SelectContent className="p-1 min-w-[200px]" side="top">
                    <div className="px-2 py-1.5 text-xs text-muted-foreground font-medium">
                      选择创作模式
                    </div>
                    {(
                      Object.entries(CREATION_MODE_CONFIG) as [
                        CreationMode,
                        (typeof CREATION_MODE_CONFIG)[CreationMode],
                      ][]
                    ).map(([key, config]) => (
                      <SelectItem key={key} value={key}>
                        <div className="flex items-center gap-3">
                          <span className="flex-shrink-0">{config.icon}</span>
                          <span className="font-medium">{config.name}</span>
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}

              {activeTheme === "knowledge" && (
                <>
                  <Badge
                    variant="secondary"
                    className="cursor-pointer hover:bg-muted font-normal h-8 px-3 gap-1"
                  >
                    <Search className="w-3.5 h-3.5 mr-1" />
                    联网搜索
                  </Badge>
                  <Select value={depth} onValueChange={setDepth}>
                    <SelectTrigger className="h-8 text-xs bg-background border-input shadow-sm w-[110px]">
                      <BrainCircuit className="w-3.5 h-3.5 mr-2 text-muted-foreground" />
                      <SelectValue placeholder="深度" />
                    </SelectTrigger>
                    <SelectContent side="top">
                      <SelectItem value="deep">深度解析</SelectItem>
                      <SelectItem value="quick">快速概览</SelectItem>
                    </SelectContent>
                  </Select>
                </>
              )}

              {activeTheme === "planning" && (
                <Badge
                  variant="outline"
                  className="h-8 font-normal text-muted-foreground gap-1"
                >
                  <Globe className="w-3.5 h-3.5 mr-1" />
                  旅行/职业/活动
                </Badge>
              )}

              {activeTheme === "poster" && (
                <>
                  <Popover
                    open={ratioPopoverOpen}
                    onOpenChange={(open) => {
                      setRatioPopoverOpen(open);
                      if (open) setStylePopoverOpen(false);
                    }}
                  >
                    <PopoverTrigger asChild>
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-8 text-xs font-normal"
                      >
                        <div className="w-3.5 h-3.5 border border-current rounded-[2px] mr-2 flex items-center justify-center text-[6px]">
                          3:4
                        </div>
                        {ratio}
                        <ChevronDown className="w-3 h-3 ml-1 opacity-50" />
                      </Button>
                    </PopoverTrigger>
                    <PopoverContent
                      className="w-64 p-2 bg-background border shadow-lg"
                      align="start"
                      side="top"
                    >
                      <div className="text-xs font-medium mb-2 px-2 text-muted-foreground">
                        宽高比
                      </div>
                      <GridSelect>
                        {["1:1", "3:4", "4:3", "9:16", "16:9", "21:9"].map(
                          (r) => (
                            <GridItem
                              key={r}
                              $active={ratio === r}
                              onClick={() => {
                                setRatio(r);
                                setRatioPopoverOpen(false);
                              }}
                            >
                              <div className="w-5 h-5 border-2 border-current rounded-sm mb-1 opacity-50"></div>
                              <span className="text-xs">{r}</span>
                            </GridItem>
                          ),
                        )}
                      </GridSelect>
                    </PopoverContent>
                  </Popover>

                  <Popover
                    open={stylePopoverOpen}
                    onOpenChange={(open) => {
                      setStylePopoverOpen(open);
                      if (open) setRatioPopoverOpen(false);
                    }}
                  >
                    <PopoverTrigger asChild>
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-8 text-xs font-normal"
                      >
                        <ColorDot $color="#3b82f6" className="mr-2" />
                        {style === "minimal"
                          ? "极简风格"
                          : style === "tech"
                            ? "科技质感"
                            : "温暖治愈"}
                        <ChevronDown className="w-3 h-3 ml-1 opacity-50" />
                      </Button>
                    </PopoverTrigger>
                    <PopoverContent
                      className="w-48 p-1 bg-background border shadow-lg"
                      align="start"
                      side="top"
                    >
                      <div className="p-1">
                        <Button
                          variant="ghost"
                          size="sm"
                          className="w-full justify-start h-8"
                          onClick={() => {
                            setStyle("minimal");
                            setStylePopoverOpen(false);
                          }}
                        >
                          <ColorDot $color="#e2e8f0" className="mr-2" />{" "}
                          极简风格
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="w-full justify-start h-8"
                          onClick={() => {
                            setStyle("tech");
                            setStylePopoverOpen(false);
                          }}
                        >
                          <ColorDot $color="#3b82f6" className="mr-2" />{" "}
                          科技质感
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="w-full justify-start h-8"
                          onClick={() => {
                            setStyle("warm");
                            setStylePopoverOpen(false);
                          }}
                        >
                          <ColorDot $color="#f59e0b" className="mr-2" />{" "}
                          温暖治愈
                        </Button>
                      </div>
                    </PopoverContent>
                  </Popover>
                </>
              )}

              <Button
                variant="outline"
                size="icon"
                className="h-8 w-8 rounded-full ml-1 bg-background shadow-sm hover:bg-muted"
              >
                <Globe className="w-4 h-4 opacity-70" />
              </Button>
            </ToolLoginLeft>

            <Button
              size="sm"
              onClick={handleSend}
              disabled={!input.trim()}
              className="bg-primary hover:bg-primary/90 text-primary-foreground h-9 px-5 rounded-xl shadow-lg shadow-primary/20 transition-all hover:scale-105 active:scale-95"
            >
              开始生成
              <ArrowRight className="h-4 w-4 ml-2" />
            </Button>
          </Toolbar>
        </InputCard>

        {/* Dynamic Inspiration/Tips based on Tab - Styled nicely */}
        <div className="w-full max-w-[800px] flex flex-wrap gap-3 justify-center">
          {RECOMMENDATIONS[activeTheme]?.map(([shortLabel, fullPrompt]) => (
            <Badge
              key={shortLabel}
              variant="secondary"
              className="px-4 py-2 text-xs font-normal cursor-pointer hover:bg-muted-foreground/10 transition-colors"
              title={fullPrompt}
              onClick={() => {
                if (onRecommendationClick) {
                  onRecommendationClick(shortLabel, fullPrompt);
                } else {
                  setInput(fullPrompt);
                  setTimeout(() => {
                    onSend(fullPrompt);
                  }, 100);
                }
              }}
            >
              {THEME_ICONS[activeTheme] || "✨"} {shortLabel}
            </Badge>
          ))}
        </div>
      </ContentWrapper>
    </Container>
  );
};
