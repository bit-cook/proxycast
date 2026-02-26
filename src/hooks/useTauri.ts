// 使用共享的 safeInvoke
import { safeInvoke } from "@/lib/dev-bridge";

export interface ServerStatus {
  running: boolean;
  host: string;
  port: number;
  requests: number;
  uptime_secs: number;
}

// TLS Configuration
export interface TlsConfig {
  enable: boolean;
  cert_path: string | null;
  key_path: string | null;
}

// Remote Management Configuration
export interface RemoteManagementConfig {
  allow_remote: boolean;
  secret_key: string | null;
  disable_control_panel: boolean;
}

// Quota Exceeded Configuration
export interface QuotaExceededConfig {
  switch_project: boolean;
  switch_preview_model: boolean;
  cooldown_seconds: number;
}

// Amp Model Mapping
export interface AmpModelMapping {
  from: string;
  to: string;
}

// Amp CLI Configuration
export interface AmpConfig {
  upstream_url: string | null;
  model_mappings: AmpModelMapping[];
  restrict_management_to_localhost: boolean;
}

// Gemini API Key Entry
export interface GeminiApiKeyEntry {
  id: string;
  api_key: string;
  base_url: string | null;
  proxy_url: string | null;
  excluded_models: string[];
  disabled: boolean;
}

// Vertex Model Alias
export interface VertexModelAlias {
  name: string;
  alias: string;
}

// Vertex API Key Entry
export interface VertexApiKeyEntry {
  id: string;
  api_key: string;
  base_url: string | null;
  models: VertexModelAlias[];
  proxy_url: string | null;
  disabled: boolean;
}

// iFlow Credential Entry
export interface IFlowCredentialEntry {
  id: string;
  token_file: string | null;
  auth_type: string;
  cookies: string | null;
  proxy_url: string | null;
  disabled: boolean;
}

// Credential Entry (OAuth)
export interface CredentialEntry {
  id: string;
  token_file: string;
  disabled: boolean;
  proxy_url: string | null;
}

// Credential Pool Configuration
export interface CredentialPoolConfig {
  kiro: CredentialEntry[];
  gemini: CredentialEntry[];
  qwen: CredentialEntry[];
  openai: ApiKeyEntry[];
  claude: ApiKeyEntry[];
  gemini_api_keys: GeminiApiKeyEntry[];
  vertex_api_keys: VertexApiKeyEntry[];
  codex: CredentialEntry[];
  iflow: IFlowCredentialEntry[];
}

// API Key Entry
export interface ApiKeyEntry {
  id: string;
  api_key: string;
  base_url: string | null;
  disabled: boolean;
  proxy_url: string | null;
}

// ============ 实验室功能配置 ============

/**
 * 截图对话功能配置
 */
export interface SmartInputConfig {
  /** 是否启用截图对话功能 */
  enabled: boolean;
  /** 触发截图的全局快捷键 */
  shortcut: string;
}

/**
 * 实验室功能配置
 */
export interface ExperimentalFeatures {
  /** 截图对话功能配置 */
  screenshot_chat: SmartInputConfig;
}

/**
 * 内容创作配置
 */
export interface ContentCreatorConfig {
  /** 启用的主题列表 */
  enabled_themes: string[];
}

/**
 * 导航栏配置
 */
export interface NavigationConfig {
  /** 启用的导航模块列表 */
  enabled_items: string[];
}

/**
 * 聊天外观配置
 */
export interface ChatAppearanceConfig {
  /** 字体大小 (12-18) */
  fontSize?: number;
  /** 消息过渡模式 */
  transitionMode?: "none" | "fadeIn" | "smooth";
  /** 气泡样式 */
  bubbleStyle?: "default" | "minimal" | "colorful";
  /** 显示头像 */
  showAvatar?: boolean;
  /** 显示时间戳 */
  showTimestamp?: boolean;
  /** 推荐点击时自动附带当前选中文本上下文 */
  append_selected_text_to_recommendation?: boolean;
}

/**
 * 记忆管理系统配置
 */
export interface MemoryConfig {
  /** 是否启用记忆功能 */
  enabled: boolean;
  /** 最大记忆条数 */
  max_entries?: number;
  /** 记忆保留天数 */
  retention_days?: number;
  /** 自动清理过期记忆 */
  auto_cleanup?: boolean;
}

/**
 * 语音服务配置
 */
export interface VoiceConfig {
  /** TTS 服务商 */
  tts_service?: "openai" | "azure" | "google" | "edge" | "macos";
  /** STT 服务商 */
  stt_service?: "openai" | "azure" | "google" | "whisper";
  /** TTS 语音 */
  tts_voice?: string;
  /** TTS 语速 (0.1-2.0) */
  tts_rate?: number;
  /** TTS 音调 (0.1-2.0) */
  tts_pitch?: number;
  /** TTS 音量 (0-1) */
  tts_volume?: number;
  /** STT 语言 */
  stt_language?: string;
  /** 自动停止录音 */
  stt_auto_stop?: boolean;
  /** 启用语音输入 */
  voice_input_enabled?: boolean;
  /** 启用语音输出 */
  voice_output_enabled?: boolean;
}

/**
 * 图像生成服务配置
 */
export interface ImageGenConfig {
  /** 默认图像生成服务 */
  default_service?: "dall_e" | "midjourney" | "stable_diffusion" | "flux";
  /** 默认图像数量 */
  default_count?: number;
  /** 默认图像尺寸 */
  default_size?:
    | "256x256"
    | "512x512"
    | "1024x1024"
    | "1792x1024"
    | "1024x1792";
  /** 默认图像质量 */
  default_quality?: "standard" | "hd";
  /** 默认图像风格 */
  default_style?: "vivid" | "natural";
  /** 启用图像增强 */
  enable_enhancement?: boolean;
  /** 自动下载生成的图像 */
  auto_download?: boolean;
}

/**
 * 助理配置
 */
export interface AssistantConfig {
  /** 默认助理 ID */
  default_assistant_id?: string;
  /** 自定义助理列表 */
  custom_assistants?: Array<{
    id: string;
    name: string;
    description?: string;
    model?: string;
    system_prompt?: string;
    temperature?: number;
    max_tokens?: number;
  }>;
  /** 启用助理自动选择 */
  auto_select?: boolean;
  /** 显示助理建议 */
  show_suggestions?: boolean;
}

/**
 * 用户资料配置
 */
export interface UserProfile {
  /** 用户头像 URL */
  avatar_url?: string;
  /** 昵称 */
  nickname?: string;
  /** 个人简介 */
  bio?: string;
  /** 邮箱 */
  email?: string;
  /** 偏好标签 */
  tags?: string[];
}

// ============ 渠道配置类型 ============

export interface TelegramBotConfig {
  enabled: boolean;
  bot_token: string;
  allowed_user_ids: string[];
  default_model?: string;
}

export interface DiscordBotConfig {
  enabled: boolean;
  bot_token: string;
  allowed_server_ids: string[];
  default_model?: string;
}

export interface FeishuBotConfig {
  enabled: boolean;
  app_id: string;
  app_secret: string;
  verification_token?: string;
  encrypt_key?: string;
  default_model?: string;
}

export interface ChannelsConfig {
  telegram: TelegramBotConfig;
  discord: DiscordBotConfig;
  feishu: FeishuBotConfig;
}

export interface Config {
  server: {
    host: string;
    port: number;
    api_key: string;
    tls: TlsConfig;
  };
  providers: {
    kiro: {
      enabled: boolean;
      credentials_path: string | null;
      region: string | null;
    };
    gemini: {
      enabled: boolean;
      credentials_path: string | null;
    };
    qwen: {
      enabled: boolean;
      credentials_path: string | null;
    };
    openai: {
      enabled: boolean;
      api_key: string | null;
      base_url: string | null;
    };
    claude: {
      enabled: boolean;
      api_key: string | null;
      base_url: string | null;
    };
  };
  default_provider: string;
  remote_management: RemoteManagementConfig;
  quota_exceeded: QuotaExceededConfig;
  ampcode: AmpConfig;
  credential_pool: CredentialPoolConfig;
  proxy_url: string | null;
  /** 关闭时最小化到托盘（而不是退出应用） */
  minimize_to_tray: boolean;
  /** 用户界面语言 ("zh" 或 "en") */
  language: string;
  /** 实验室功能配置 */
  experimental?: ExperimentalFeatures;
  /** 内容创作配置 */
  content_creator?: ContentCreatorConfig;
  /** 导航栏配置 */
  navigation?: NavigationConfig;
  /** 聊天外观配置 */
  chat_appearance?: ChatAppearanceConfig;
  /** 记忆管理配置 */
  memory?: MemoryConfig;
  /** 语音服务配置 */
  voice?: VoiceConfig;
  /** 图像生成服务配置 */
  image_gen?: ImageGenConfig;
  /** 助理配置 */
  assistant?: AssistantConfig;
  /** 用户资料 */
  user_profile?: UserProfile;
  /** 渠道配置（Telegram / Discord / 飞书 Bot） */
  channels?: ChannelsConfig;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

export async function startServer(): Promise<string> {
  return safeInvoke("start_server");
}

export async function stopServer(): Promise<string> {
  return safeInvoke("stop_server");
}

export async function getServerStatus(): Promise<ServerStatus> {
  return safeInvoke("get_server_status");
}

export async function getConfig(): Promise<Config> {
  return safeInvoke("get_config");
}

export async function saveConfig(config: Config): Promise<void> {
  return safeInvoke("save_config", { config });
}

export async function getDefaultProvider(): Promise<string> {
  return safeInvoke("get_default_provider");
}

export async function setDefaultProvider(provider: string): Promise<string> {
  return safeInvoke("set_default_provider", { provider });
}

/**
 * 更新 Provider 的环境变量
 *
 * 当用户在团队共享网关页面选择一个 API Key Provider 时调用
 * 会更新 ~/.claude/settings.json 和 shell 配置文件中的环境变量
 *
 * @param providerType Provider 类型（如 "anthropic", "openai", "gemini"）
 * @param apiHost Provider 的 API Host
 * @param apiKey 可选的 API Key
 */
export async function updateProviderEnvVars(
  providerType: string,
  apiHost: string,
  apiKey?: string,
): Promise<void> {
  return safeInvoke("update_provider_env_vars", {
    providerType,
    apiHost,
    apiKey: apiKey || null,
  });
}

export async function refreshKiroToken(): Promise<string> {
  return safeInvoke("refresh_kiro_token");
}

export async function reloadCredentials(): Promise<string> {
  return safeInvoke("reload_credentials");
}

export async function getLogs(): Promise<LogEntry[]> {
  try {
    return await safeInvoke("get_logs");
  } catch {
    return [];
  }
}

export async function clearLogs(): Promise<void> {
  try {
    await safeInvoke("clear_logs");
  } catch {
    // ignore
  }
}

export interface TestResult {
  success: boolean;
  status: number;
  body: string;
  time_ms: number;
}

export async function testApi(
  method: string,
  path: string,
  body: string | null,
  auth: boolean,
): Promise<TestResult> {
  return safeInvoke("test_api", { method, path, body, auth });
}

export interface KiroCredentialStatus {
  loaded: boolean;
  has_access_token: boolean;
  has_refresh_token: boolean;
  region: string | null;
  auth_method: string | null;
  expires_at: string | null;
  creds_path: string;
}

export async function getKiroCredentials(): Promise<KiroCredentialStatus> {
  return safeInvoke("get_kiro_credentials");
}

export interface EnvVariable {
  key: string;
  value: string;
  masked: string;
}

export async function getEnvVariables(): Promise<EnvVariable[]> {
  return safeInvoke("get_env_variables");
}

export async function getTokenFileHash(): Promise<string> {
  return safeInvoke("get_token_file_hash");
}

export interface CheckResult {
  changed: boolean;
  new_hash: string;
  reloaded: boolean;
}

export async function checkAndReloadCredentials(
  lastHash: string,
): Promise<CheckResult> {
  return safeInvoke("check_and_reload_credentials", { last_hash: lastHash });
}

// ============ Gemini Provider ============

export interface GeminiCredentialStatus {
  loaded: boolean;
  has_access_token: boolean;
  has_refresh_token: boolean;
  expiry_date: number | null;
  is_valid: boolean;
  creds_path: string;
}

export async function getGeminiCredentials(): Promise<GeminiCredentialStatus> {
  return safeInvoke("get_gemini_credentials");
}

export async function reloadGeminiCredentials(): Promise<string> {
  return safeInvoke("reload_gemini_credentials");
}

export async function refreshGeminiToken(): Promise<string> {
  return safeInvoke("refresh_gemini_token");
}

export async function getGeminiEnvVariables(): Promise<EnvVariable[]> {
  return safeInvoke("get_gemini_env_variables");
}

export async function getGeminiTokenFileHash(): Promise<string> {
  return safeInvoke("get_gemini_token_file_hash");
}

export async function checkAndReloadGeminiCredentials(
  lastHash: string,
): Promise<CheckResult> {
  return safeInvoke("check_and_reload_gemini_credentials", {
    last_hash: lastHash,
  });
}

// ============ Qwen Provider ============

export interface QwenCredentialStatus {
  loaded: boolean;
  has_access_token: boolean;
  has_refresh_token: boolean;
  expiry_date: number | null;
  is_valid: boolean;
  creds_path: string;
}

export async function getQwenCredentials(): Promise<QwenCredentialStatus> {
  return safeInvoke("get_qwen_credentials");
}

export async function reloadQwenCredentials(): Promise<string> {
  return safeInvoke("reload_qwen_credentials");
}

export async function refreshQwenToken(): Promise<string> {
  return safeInvoke("refresh_qwen_token");
}

export async function getQwenEnvVariables(): Promise<EnvVariable[]> {
  return safeInvoke("get_qwen_env_variables");
}

export async function getQwenTokenFileHash(): Promise<string> {
  return safeInvoke("get_qwen_token_file_hash");
}

export async function checkAndReloadQwenCredentials(
  lastHash: string,
): Promise<CheckResult> {
  return safeInvoke("check_and_reload_qwen_credentials", {
    last_hash: lastHash,
  });
}

// ============ OpenAI Custom Provider ============

export interface OpenAICustomStatus {
  enabled: boolean;
  has_api_key: boolean;
  base_url: string;
}

export async function getOpenAICustomStatus(): Promise<OpenAICustomStatus> {
  return safeInvoke("get_openai_custom_status");
}

export async function setOpenAICustomConfig(
  apiKey: string | null,
  baseUrl: string | null,
  enabled: boolean,
): Promise<string> {
  return safeInvoke("set_openai_custom_config", {
    api_key: apiKey,
    base_url: baseUrl,
    enabled,
  });
}

// ============ Claude Custom Provider ============

export interface ClaudeCustomStatus {
  enabled: boolean;
  has_api_key: boolean;
  base_url: string;
}

export async function getClaudeCustomStatus(): Promise<ClaudeCustomStatus> {
  return safeInvoke("get_claude_custom_status");
}

export async function setClaudeCustomConfig(
  apiKey: string | null,
  baseUrl: string | null,
  enabled: boolean,
): Promise<string> {
  return safeInvoke("set_claude_custom_config", {
    api_key: apiKey,
    base_url: baseUrl,
    enabled,
  });
}

// ============ Models ============

export interface ModelInfo {
  id: string;
  object: string;
  owned_by: string;
}

export async function getAvailableModels(): Promise<ModelInfo[]> {
  return safeInvoke("get_available_models");
}

// ============ API Compatibility Check ============

export interface ApiCheckResult {
  model: string;
  available: boolean;
  status: number;
  error_type: string | null;
  error_message: string | null;
  time_ms: number;
}

export interface ApiCompatibilityResult {
  provider: string;
  overall_status: string;
  checked_at: string;
  results: ApiCheckResult[];
  warnings: string[];
}

export async function checkApiCompatibility(
  provider: string,
): Promise<ApiCompatibilityResult> {
  return safeInvoke("check_api_compatibility", { provider });
}

// ============ Endpoint Provider Configuration ============

/**
 * 端点 Provider 配置
 * 为不同客户端类型配置不同的 LLM Provider
 */
export interface EndpointProvidersConfig {
  /** Cursor 客户端使用的 Provider */
  cursor?: string | null;
  /** Claude Code 客户端使用的 Provider */
  claude_code?: string | null;
  /** Codex 客户端使用的 Provider */
  codex?: string | null;
  /** Windsurf 客户端使用的 Provider */
  windsurf?: string | null;
  /** Kiro 客户端使用的 Provider */
  kiro?: string | null;
  /** 其他客户端使用的 Provider */
  other?: string | null;
}

/**
 * 获取端点 Provider 配置
 * @returns 端点 Provider 配置对象
 */
export async function getEndpointProviders(): Promise<EndpointProvidersConfig> {
  return safeInvoke("get_endpoint_providers");
}

/**
 * 设置端点 Provider 配置
 * @param clientType 客户端类型 (cursor, claude_code, codex, windsurf, kiro, other)
 * @param provider Provider 名称，传 null 表示使用默认 Provider
 * @returns 设置后的 Provider 名称
 */
export async function setEndpointProvider(
  clientType: string,
  provider: string | null,
): Promise<string> {
  return safeInvoke("set_endpoint_provider", {
    endpoint: clientType,
    provider,
  });
}

// Network Info
export interface NetworkInfo {
  localhost: string;
  lan_ip: string | null;
  all_ips: string[];
}

/**
 * 获取本地网络信息
 * @returns 本地和内网 IP 地址
 */
export async function getNetworkInfo(): Promise<NetworkInfo> {
  return safeInvoke("get_network_info");
}

// ============ 实验室功能 API ============

/**
 * 获取实验室功能配置
 * @returns 实验室功能配置对象
 */
export async function getExperimentalConfig(): Promise<ExperimentalFeatures> {
  return safeInvoke("get_experimental_config");
}

/**
 * 保存实验室功能配置
 * @param config 实验室功能配置对象
 */
export async function saveExperimentalConfig(
  config: ExperimentalFeatures,
): Promise<void> {
  return safeInvoke("save_experimental_config", {
    experimentalConfig: config,
  });
}

/**
 * 验证快捷键格式
 * @param shortcut 快捷键字符串
 * @returns 是否有效
 */
export async function validateShortcut(shortcut: string): Promise<boolean> {
  return safeInvoke("validate_shortcut", { shortcutStr: shortcut });
}

/**
 * 更新截图快捷键
 * @param shortcut 新的快捷键字符串
 */
export async function updateScreenshotShortcut(
  shortcut: string,
): Promise<void> {
  return safeInvoke("update_screenshot_shortcut", { newShortcut: shortcut });
}

// ============ 使用统计 API ============

export interface UsageStatsResponse {
  total_conversations: number;
  total_messages: number;
  total_tokens: number;
  total_time_minutes: number;
  monthly_conversations: number;
  monthly_messages: number;
  monthly_tokens: number;
  today_conversations: number;
  today_messages: number;
  today_tokens: number;
}

export interface ModelUsage {
  model: string;
  conversations: number;
  tokens: number;
  percentage: number;
}

export interface DailyUsage {
  date: string;
  conversations: number;
  tokens: number;
}

/**
 * 获取使用统计数据
 * @param timeRange 时间范围 (week/month/all)
 */
export async function getUsageStats(
  timeRange: string,
): Promise<UsageStatsResponse> {
  return safeInvoke("get_usage_stats", { timeRange });
}

/**
 * 获取模型使用排行
 * @param timeRange 时间范围 (week/month/all)
 */
export async function getModelUsageRanking(
  timeRange: string,
): Promise<ModelUsage[]> {
  return safeInvoke("get_model_usage_ranking", { timeRange });
}

/**
 * 获取每日使用趋势
 * @param timeRange 时间范围 (week/month/all)
 */
export async function getDailyUsageTrends(
  timeRange: string,
): Promise<DailyUsage[]> {
  return safeInvoke("get_daily_usage_trends", { timeRange });
}

// ============ 记忆管理 API ============

export interface MemoryStatsResponse {
  total_entries: number;
  storage_used: number;
  memory_count: number;
}

export interface MemoryCategoryStat {
  category: "identity" | "context" | "preference" | "experience" | "activity";
  count: number;
}

export interface MemoryEntryPreview {
  id: string;
  session_id: string;
  file_type: string;
  category: "identity" | "context" | "preference" | "experience" | "activity";
  title: string;
  summary: string;
  updated_at: number;
  tags: string[];
}

export interface MemoryOverviewResponse {
  stats: MemoryStatsResponse;
  categories: MemoryCategoryStat[];
  entries: MemoryEntryPreview[];
}

export interface CleanupMemoryResult {
  cleaned_entries: number;
  freed_space: number;
}

export interface MemoryAnalysisResult {
  analyzed_sessions: number;
  analyzed_messages: number;
  generated_entries: number;
  deduplicated_entries: number;
}

/**
 * 获取记忆统计信息
 */
export async function getMemoryStats(): Promise<MemoryStatsResponse> {
  return safeInvoke("get_conversation_memory_stats");
}

/**
 * 获取记忆总览（含分类与条目）
 */
export async function getMemoryOverview(
  limit?: number,
): Promise<MemoryOverviewResponse> {
  return safeInvoke("get_conversation_memory_overview", { limit });
}

/**
 * 请求记忆分析（从历史会话提取记忆）
 */
export async function requestMemoryAnalysis(
  fromTimestamp?: number,
  toTimestamp?: number,
): Promise<MemoryAnalysisResult> {
  return safeInvoke("request_conversation_memory_analysis", {
    fromTimestamp,
    toTimestamp,
  });
}

/**
 * 清理过期记忆
 */
export async function cleanupMemory(): Promise<CleanupMemoryResult> {
  return safeInvoke("cleanup_conversation_memory");
}

// ============ 语音测试 API ============

export interface TtsTestResult {
  success: boolean;
  error: string | null;
  audio_path: string | null;
}

export interface VoiceOption {
  id: string;
  name: string;
  language: string;
}

/**
 * 测试 TTS 语音合成
 * @param service TTS 服务名称
 * @param voice 语音 ID
 */
export async function testTts(
  service: string,
  voice: string,
): Promise<TtsTestResult> {
  return safeInvoke("test_tts", { service, voice });
}

/**
 * 获取可用的语音列表
 * @param service TTS 服务名称
 */
export async function getAvailableVoices(
  service: string,
): Promise<VoiceOption[]> {
  return safeInvoke("get_available_voices", { service });
}

// ============ 文件上传 API ============

export interface UploadResult {
  url: string;
  size: number;
}

/**
 * 上传用户头像
 * @param filePath 文件路径
 */
export async function uploadAvatar(filePath: string): Promise<UploadResult> {
  return safeInvoke("upload_avatar", { filePath });
}

/**
 * 删除用户头像
 * @param url 头像 URL
 */
export async function deleteAvatar(url: string): Promise<void> {
  return safeInvoke("delete_avatar", { url });
}
