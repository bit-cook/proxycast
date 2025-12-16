import { invoke } from "@tauri-apps/api/core";

// Retry configuration
export interface RetryConfig {
  max_retries: number;
  base_delay_ms: number;
  max_delay_ms: number;
  retryable_codes: number[];
}

// Failover configuration
export interface FailoverConfig {
  auto_switch: boolean;
  switch_on_quota: boolean;
}

// Switch log entry
export interface SwitchLogEntry {
  from_provider: string;
  to_provider: string;
  failure_type: string;
  timestamp: string;
}

export const resilienceApi = {
  // Retry config
  async getRetryConfig(): Promise<RetryConfig> {
    return invoke("get_retry_config");
  },

  async updateRetryConfig(config: RetryConfig): Promise<void> {
    return invoke("update_retry_config", { config });
  },

  // Failover config
  async getFailoverConfig(): Promise<FailoverConfig> {
    return invoke("get_failover_config");
  },

  async updateFailoverConfig(config: FailoverConfig): Promise<void> {
    return invoke("update_failover_config", { config });
  },

  // Switch log
  async getSwitchLog(): Promise<SwitchLogEntry[]> {
    return invoke("get_switch_log");
  },

  async clearSwitchLog(): Promise<void> {
    return invoke("clear_switch_log");
  },
};
