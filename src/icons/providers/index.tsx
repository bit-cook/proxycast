/**
 * @file Provider 图标组件
 * @description 统一的 Provider 图标组件，支持所有 System Provider
 * @module icons/providers
 *
 * **Feature: provider-ui-refactor**
 * **Validates: Requirements 10.1, 10.2, 10.4**
 */

import React, { useMemo, ComponentType, SVGProps } from "react";
import { cn } from "@/lib/utils";
import { providerTypeToIcon } from "./utils";

// ============================================================================
// SVG 图标导入 - 使用 SVGR
// ============================================================================

// 现有图标
import AwsIcon from "./aws.svg?react";
import GeminiIcon from "./gemini.svg?react";
import AnthropicIcon from "./anthropic.svg?react";
import ClaudeIcon from "./claude.svg?react";
import QwenIcon from "./qwen.svg?react";
import GoogleIcon from "./google.svg?react";
import OpenaiIcon from "./openai.svg?react";
import AlibabaIcon from "./alibaba.svg?react";
import CopilotIcon from "./copilot.svg?react";
import AmpIcon from "./amp.svg?react";
import KiroIcon from "./kiro.svg?react";
import DeepseekIcon from "./deepseek.svg?react";
import ZhipuIcon from "./zhipu.svg?react";
import KimiIcon from "./kimi.svg?react";
import MinimaxIcon from "./minimax.svg?react";
import DoubaoIcon from "./doubao.svg?react";
import AzureIcon from "./azure.svg?react";
import antigravityIconUrl from "./antigravity.svg?url";
import ProxycastIcon from "./proxycast.svg?react";

// 新增图标 - 主流 AI
import PerplexityIcon from "./perplexity.svg?react";
import MoonshotIcon from "./moonshot.svg?react";
import GrokIcon from "./grok.svg?react";
import GroqIcon from "./groq.svg?react";
import MistralIcon from "./mistral.svg?react";
import CohereIcon from "./cohere.svg?react";

// 新增图标 - 国内 AI
import BaiduIcon from "./baidu.svg?react";
import YiIcon from "./yi.svg?react";
import BaichuanIcon from "./baichuan.svg?react";
import HunyuanIcon from "./hunyuan.svg?react";
import StepfunIcon from "./stepfun.svg?react";
import TencentIcon from "./tencent.svg?react";
import InfiniIcon from "./infini.svg?react";
import XirangIcon from "./xirang.svg?react";
import MimoIcon from "./mimo.svg?react";
import ModelscopeIcon from "./modelscope.svg?react";
import ZhinaoIcon from "./zhinao.svg?react";
import DashscopeIcon from "./dashscope.svg?react";

// 新增图标 - 云服务
import VertexaiIcon from "./vertexai.svg?react";
import BedrockIcon from "./bedrock.svg?react";
import GithubIcon from "./github.svg?react";

// 新增图标 - API 聚合
import SiliconIcon from "./silicon.svg?react";
import OpenrouterIcon from "./openrouter.svg?react";
import Ai302Icon from "./302ai.svg?react";
import AihubmixIcon from "./aihubmix.svg?react";
import TogetherIcon from "./together.svg?react";
import PpioIcon from "./ppio.svg?react";
import HyperbolicIcon from "./hyperbolic.svg?react";
import CerebrasIcon from "./cerebras.svg?react";
import NvidiaIcon from "./nvidia.svg?react";
import FireworksIcon from "./fireworks.svg?react";
import TokenfluxIcon from "./tokenflux.svg?react";
import CephalonIcon from "./cephalon.svg?react";
import Ph8Icon from "./ph8.svg?react";
import QiniuIcon from "./qiniu.svg?react";
import LanyunIcon from "./lanyun.svg?react";
import SophnetIcon from "./sophnet.svg?react";
import BurncloudIcon from "./burncloud.svg?react";
import DmxapiIcon from "./dmxapi.svg?react";
import LongcatIcon from "./longcat.svg?react";
import AlayanewIcon from "./alayanew.svg?react";
import AionlyIcon from "./aionly.svg?react";
import OcoolaiIcon from "./ocoolai.svg?react";
import VercelIcon from "./vercel.svg?react";
import PoeIcon from "./poe.svg?react";
import HuggingfaceIcon from "./huggingface.svg?react";

// 新增图标 - 本地服务
import OllamaIcon from "./ollama.svg?react";
import LmstudioIcon from "./lmstudio.svg?react";
import NewapiIcon from "./newapi.svg?react";
import GpustackIcon from "./gpustack.svg?react";
import OvmsIcon from "./ovms.svg?react";

// 新增图标 - 专用服务
import JinaIcon from "./jina.svg?react";
import VoyageaiIcon from "./voyageai.svg?react";
import CherryinIcon from "./cherryin.svg?react";

// 自定义 Provider 图标
import CustomIcon from "./custom.svg?react";

const AntigravityIcon: React.FC<SVGProps<SVGSVGElement>> = ({
  width = "1em",
  height = "1em",
}) => {
  return (
    <img
      src={antigravityIconUrl}
      width={width}
      height={height}
      alt="Antigravity"
    />
  );
};

// ============================================================================
// 图标组件映射
// ============================================================================

/**
 * 图标组件映射表
 * 将图标名称映射到对应的 SVG 组件
 */
const iconComponents: Record<string, ComponentType<SVGProps<SVGSVGElement>>> = {
  // 现有图标
  aws: AwsIcon,
  gemini: GeminiIcon,
  anthropic: AnthropicIcon,
  claude: ClaudeIcon,
  qwen: QwenIcon,
  google: GoogleIcon,
  openai: OpenaiIcon,
  alibaba: AlibabaIcon,
  copilot: CopilotIcon,
  amp: AmpIcon,
  kiro: KiroIcon,
  deepseek: DeepseekIcon,
  zhipu: ZhipuIcon,
  kimi: KimiIcon,
  minimax: MinimaxIcon,
  doubao: DoubaoIcon,
  azure: AzureIcon,
  antigravity: AntigravityIcon,
  proxycast: ProxycastIcon,

  // 主流 AI
  perplexity: PerplexityIcon,
  moonshot: MoonshotIcon,
  grok: GrokIcon,
  groq: GroqIcon,
  mistral: MistralIcon,
  cohere: CohereIcon,

  // 国内 AI
  baidu: BaiduIcon,
  yi: YiIcon,
  baichuan: BaichuanIcon,
  hunyuan: HunyuanIcon,
  stepfun: StepfunIcon,
  tencent: TencentIcon,
  infini: InfiniIcon,
  xirang: XirangIcon,
  mimo: MimoIcon,
  modelscope: ModelscopeIcon,
  zhinao: ZhinaoIcon,
  dashscope: DashscopeIcon,

  // 云服务
  vertexai: VertexaiIcon,
  bedrock: BedrockIcon,
  github: GithubIcon,

  // API 聚合
  silicon: SiliconIcon,
  openrouter: OpenrouterIcon,
  "302ai": Ai302Icon,
  aihubmix: AihubmixIcon,
  together: TogetherIcon,
  ppio: PpioIcon,
  hyperbolic: HyperbolicIcon,
  cerebras: CerebrasIcon,
  nvidia: NvidiaIcon,
  fireworks: FireworksIcon,
  tokenflux: TokenfluxIcon,
  cephalon: CephalonIcon,
  ph8: Ph8Icon,
  qiniu: QiniuIcon,
  lanyun: LanyunIcon,
  sophnet: SophnetIcon,
  burncloud: BurncloudIcon,
  dmxapi: DmxapiIcon,
  longcat: LongcatIcon,
  alayanew: AlayanewIcon,
  aionly: AionlyIcon,
  ocoolai: OcoolaiIcon,
  vercel: VercelIcon,
  poe: PoeIcon,
  huggingface: HuggingfaceIcon,

  // 本地服务
  ollama: OllamaIcon,
  lmstudio: LmstudioIcon,
  newapi: NewapiIcon,
  gpustack: GpustackIcon,
  ovms: OvmsIcon,

  // 专用服务
  jina: JinaIcon,
  voyageai: VoyageaiIcon,
  cherryin: CherryinIcon,

  // 自定义
  custom: CustomIcon,
};

// ============================================================================
// ProviderIcon 组件
// ============================================================================

interface ProviderIconProps {
  /** Provider 类型或 ID */
  providerType: string;
  /** 图标大小，支持数字（px）或字符串 */
  size?: number | string;
  /** 额外的 CSS 类名 */
  className?: string;
  /** 是否显示 fallback（首字母缩写） */
  showFallback?: boolean;
}

/**
 * Provider 图标组件
 *
 * 根据 Provider 类型或 ID 显示对应的图标。
 * 如果没有对应图标且 showFallback 为 true，则显示首字母缩写。
 *
 * @example
 * ```tsx
 * <ProviderIcon providerType="openai" size={24} />
 * <ProviderIcon providerType="deepseek" size="1.5rem" />
 * <ProviderIcon providerType="custom-provider" showFallback />
 * ```
 */
export const ProviderIcon: React.FC<ProviderIconProps> = ({
  providerType,
  size = 24,
  className,
  showFallback = true,
}) => {
  const iconName = providerTypeToIcon[providerType] || providerType;
  const IconComponent = iconComponents[iconName];

  const sizeStyle = useMemo(() => {
    const sizeValue = typeof size === "number" ? `${size}px` : size;
    return {
      width: sizeValue,
      height: sizeValue,
      fontSize: sizeValue,
      lineHeight: 1,
    };
  }, [size]);

  if (IconComponent) {
    return (
      <span
        className={cn(
          "inline-flex items-center justify-center flex-shrink-0",
          className,
        )}
        style={sizeStyle}
      >
        <IconComponent width="1em" height="1em" />
      </span>
    );
  }

  // Fallback：显示首字母
  if (showFallback) {
    const initials = providerType
      .split(/[-_]/)
      .map((word) => word[0])
      .join("")
      .toUpperCase()
      .slice(0, 2);
    const fallbackFontSize =
      typeof size === "number" ? `${Math.max(size * 0.5, 12)}px` : "0.5em";
    return (
      <span
        className={cn(
          "inline-flex items-center justify-center flex-shrink-0 rounded-lg",
          "bg-muted text-muted-foreground font-semibold",
          className,
        )}
        style={sizeStyle}
      >
        <span style={{ fontSize: fallbackFontSize }}>{initials}</span>
      </span>
    );
  }

  return null;
};

// ============================================================================
// 导出
// ============================================================================

export { iconComponents };
export type { ProviderIconProps };
