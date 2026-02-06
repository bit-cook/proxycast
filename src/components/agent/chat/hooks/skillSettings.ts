/**
 * Skill 执行配置
 *
 * 管理 slash skill 执行时的 Provider 覆盖策略与工具兼容 Provider 列表。
 */

export type SkillProviderOverrideMode =
  | "compatible_only"
  | "always_current"
  | "auto_fallback";

const STORAGE_KEY_MODE = "proxycast_skill_provider_override_mode";
const STORAGE_KEY_PROVIDERS = "proxycast_skill_tool_compatible_providers";

export const DEFAULT_SKILL_TOOL_COMPATIBLE_PROVIDERS = [
  "anthropic",
  "claude",
  "claude_oauth",
  "openai",
  "gemini",
  "kiro",
  "antigravity",
];

export const DEFAULT_SKILL_PROVIDER_OVERRIDE_MODE: SkillProviderOverrideMode =
  "compatible_only";

export function isSkillProviderOverrideMode(
  value: string,
): value is SkillProviderOverrideMode {
  return (
    value === "compatible_only" ||
    value === "always_current" ||
    value === "auto_fallback"
  );
}

export function getSkillProviderOverrideMode(): SkillProviderOverrideMode {
  const stored = localStorage.getItem(STORAGE_KEY_MODE);
  if (stored && isSkillProviderOverrideMode(stored)) {
    return stored;
  }
  return DEFAULT_SKILL_PROVIDER_OVERRIDE_MODE;
}

export function setSkillProviderOverrideMode(
  mode: SkillProviderOverrideMode,
): void {
  localStorage.setItem(STORAGE_KEY_MODE, mode);
}

function normalizeProviderList(providers: string[]): string[] {
  const normalized = providers
    .map((provider) => provider.toLowerCase().trim())
    .filter((provider) => provider.length > 0);
  return Array.from(new Set(normalized));
}

export function getSkillToolCompatibleProviders(): string[] {
  const raw = localStorage.getItem(STORAGE_KEY_PROVIDERS);
  if (!raw) {
    return DEFAULT_SKILL_TOOL_COMPATIBLE_PROVIDERS;
  }

  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) {
      const normalized = normalizeProviderList(
        parsed.filter((item): item is string => typeof item === "string"),
      );
      if (normalized.length > 0) {
        return normalized;
      }
    }
  } catch {
    const normalized = normalizeProviderList(raw.split(","));
    if (normalized.length > 0) {
      return normalized;
    }
  }

  return DEFAULT_SKILL_TOOL_COMPATIBLE_PROVIDERS;
}

export function setSkillToolCompatibleProviders(providers: string[]): void {
  const normalized = normalizeProviderList(providers);
  const finalProviders =
    normalized.length > 0
      ? normalized
      : DEFAULT_SKILL_TOOL_COMPATIBLE_PROVIDERS;

  localStorage.setItem(STORAGE_KEY_PROVIDERS, JSON.stringify(finalProviders));
}

export function resetSkillProviderSettings(): void {
  setSkillProviderOverrideMode(DEFAULT_SKILL_PROVIDER_OVERRIDE_MODE);
  setSkillToolCompatibleProviders(DEFAULT_SKILL_TOOL_COMPATIBLE_PROVIDERS);
}
