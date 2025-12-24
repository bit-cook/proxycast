import { invoke } from "@tauri-apps/api/core";

export interface Provider {
  id: string;
  app_type: string;
  name: string;
  settings_config: Record<string, unknown>;
  category?: string;
  icon?: string;
  icon_color?: string;
  notes?: string;
  created_at?: number;
  sort_index?: number;
  is_current: boolean;
}

// proxycast 保留用于内部配置存储，但不在 UI 的 Tab 中显示
export type AppType = "claude" | "codex" | "gemini" | "proxycast";

// 同步状态枚举
export type SyncStatus = "InSync" | "OutOfSync" | "Conflict";

// 配置冲突信息
export interface ConfigConflict {
  field: string;
  local_value: string;
  external_value: string;
}

// 同步检查结果
export interface SyncCheckResult {
  status: SyncStatus;
  current_provider: string;
  external_provider: string;
  last_modified?: string;
  conflicts: ConfigConflict[];
}

export const switchApi = {
  getProviders: (appType: AppType): Promise<Provider[]> =>
    invoke("get_switch_providers", { appType }),

  getCurrentProvider: (appType: AppType): Promise<Provider | null> =>
    invoke("get_current_switch_provider", { appType }),

  addProvider: (provider: Provider): Promise<void> =>
    invoke("add_switch_provider", { provider }),

  updateProvider: (provider: Provider): Promise<void> =>
    invoke("update_switch_provider", { provider }),

  deleteProvider: (appType: AppType, id: string): Promise<void> =>
    invoke("delete_switch_provider", { appType, id }),

  switchProvider: (appType: AppType, id: string): Promise<void> =>
    invoke("switch_provider", { appType, id }),

  /** 读取当前生效的配置（从实际配置文件读取） */
  readLiveSettings: (appType: AppType): Promise<Record<string, unknown>> =>
    invoke("read_live_provider_settings", { appType }),

  /** 检查配置同步状态 */
  checkConfigSync: (appType: AppType): Promise<SyncCheckResult> =>
    invoke("check_config_sync_status", { appType }),

  /** 从外部配置同步到 ProxyCast */
  syncFromExternal: (appType: AppType): Promise<string> =>
    invoke("sync_from_external_config", { appType }),
};
