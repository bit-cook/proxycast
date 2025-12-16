import { invoke } from "@tauri-apps/api/core";

// Config types matching Rust backend
export interface ServerConfig {
  host: string;
  port: number;
  api_key: string;
}

export interface ProviderConfig {
  enabled: boolean;
  credentials_path?: string;
  region?: string;
  project_id?: string;
}

export interface CustomProviderConfig {
  enabled: boolean;
  api_key?: string;
  base_url?: string;
}

export interface ProvidersConfig {
  kiro: ProviderConfig;
  gemini: ProviderConfig;
  qwen: ProviderConfig;
  openai: CustomProviderConfig;
  claude: CustomProviderConfig;
}

export interface RoutingRuleConfig {
  pattern: string;
  provider: string;
  priority: number;
}

export interface RoutingConfig {
  default_provider: string;
  rules: RoutingRuleConfig[];
  model_aliases: Record<string, string>;
  exclusions: Record<string, string[]>;
}

export interface RetrySettings {
  max_retries: number;
  base_delay_ms: number;
  max_delay_ms: number;
  auto_switch_provider: boolean;
}

export interface LoggingConfig {
  enabled: boolean;
  level: string;
  retention_days: number;
  include_request_body: boolean;
}

export interface Config {
  server: ServerConfig;
  providers: ProvidersConfig;
  default_provider: string;
  routing: RoutingConfig;
  retry: RetrySettings;
  logging: LoggingConfig;
}

// Export result
export interface ExportResult {
  content: string;
  suggested_filename: string;
}

// Import result
export interface ImportResult {
  success: boolean;
  config: Config;
  warnings: string[];
}

// Config path info
export interface ConfigPathInfo {
  yaml_path: string;
  json_path: string;
  yaml_exists: boolean;
  json_exists: boolean;
}

export const configApi = {
  // Export config to YAML
  async exportConfig(
    config: Config,
    redactSecrets: boolean,
  ): Promise<ExportResult> {
    return invoke("export_config", { config, redactSecrets });
  },

  // Validate YAML config
  async validateConfigYaml(yamlContent: string): Promise<Config> {
    return invoke("validate_config_yaml", { yamlContent });
  },

  // Import config from YAML
  async importConfig(
    currentConfig: Config,
    yamlContent: string,
    merge: boolean,
  ): Promise<ImportResult> {
    return invoke("import_config", { currentConfig, yamlContent, merge });
  },

  // Get config file paths
  async getConfigPaths(): Promise<ConfigPathInfo> {
    return invoke("get_config_paths");
  },
};
