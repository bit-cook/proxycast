import { invoke } from "@tauri-apps/api/core";

// Injection mode
export type InjectionMode = "merge" | "override";

// Injection rule
export interface InjectionRule {
  id: string;
  pattern: string;
  parameters: Record<string, unknown>;
  mode: InjectionMode;
  priority: number;
  enabled: boolean;
}

// Injection configuration
export interface InjectionConfig {
  enabled: boolean;
  rules: InjectionRule[];
}

export const injectionApi = {
  // Get injection configuration
  async getInjectionConfig(): Promise<InjectionConfig> {
    return invoke("get_injection_config");
  },

  // Set injection enabled
  async setInjectionEnabled(enabled: boolean): Promise<void> {
    return invoke("set_injection_enabled", { enabled });
  },

  // Add injection rule
  async addInjectionRule(rule: InjectionRule): Promise<void> {
    return invoke("add_injection_rule", { rule });
  },

  // Remove injection rule
  async removeInjectionRule(id: string): Promise<void> {
    return invoke("remove_injection_rule", { id });
  },

  // Update injection rule
  async updateInjectionRule(id: string, rule: InjectionRule): Promise<void> {
    return invoke("update_injection_rule", { id, rule });
  },

  // Get all injection rules
  async getInjectionRules(): Promise<InjectionRule[]> {
    return invoke("get_injection_rules");
  },
};
