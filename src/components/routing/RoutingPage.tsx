import { useState, useEffect, forwardRef, useImperativeHandle } from "react";
import { RefreshCw, Route } from "lucide-react";
import { ModelMapping } from "./ModelMapping";
import { RoutingRules } from "./RoutingRules";
import { ExclusionList } from "./ExclusionList";
import { InjectionRules } from "./InjectionRules";
import { HelpTip } from "@/components/HelpTip";
import { routerApi } from "@/lib/api/router";
import { injectionApi } from "@/lib/api/injection";
import type { ModelAlias, RoutingRule, ProviderType } from "@/lib/api/router";
import type { InjectionRule } from "@/lib/api/injection";

export interface RoutingPageRef {
  refresh: () => void;
}

type TabType = "aliases" | "rules" | "exclusions" | "injection";

export const RoutingPage = forwardRef<RoutingPageRef>((_props, ref) => {
  const [activeTab, setActiveTab] = useState<TabType>("aliases");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Data state
  const [aliases, setAliases] = useState<ModelAlias[]>([]);
  const [rules, setRules] = useState<RoutingRule[]>([]);
  const [exclusions, setExclusions] = useState<Record<ProviderType, string[]>>(
    {} as Record<ProviderType, string[]>,
  );
  const [injectionRules, setInjectionRules] = useState<InjectionRule[]>([]);
  const [injectionEnabled, setInjectionEnabled] = useState(false);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [aliasesData, rulesData, exclusionsData, injectionConfig] =
        await Promise.all([
          routerApi.getModelAliases(),
          routerApi.getRoutingRules(),
          routerApi.getExclusions(),
          injectionApi.getInjectionConfig(),
        ]);
      setAliases(aliasesData);
      setRules(rulesData);
      setExclusions(exclusionsData);
      setInjectionRules(injectionConfig.rules);
      setInjectionEnabled(injectionConfig.enabled);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  useImperativeHandle(ref, () => ({
    refresh,
  }));

  useEffect(() => {
    refresh();
  }, []);

  // Alias handlers
  const handleAddAlias = async (alias: string, actual: string) => {
    await routerApi.addModelAlias(alias, actual);
    await refresh();
  };

  const handleRemoveAlias = async (alias: string) => {
    await routerApi.removeModelAlias(alias);
    await refresh();
  };

  // Rule handlers
  const handleAddRule = async (rule: RoutingRule) => {
    await routerApi.addRoutingRule(rule);
    await refresh();
  };

  const handleRemoveRule = async (pattern: string) => {
    await routerApi.removeRoutingRule(pattern);
    await refresh();
  };

  const handleUpdateRule = async (pattern: string, rule: RoutingRule) => {
    await routerApi.updateRoutingRule(pattern, rule);
    await refresh();
  };

  // Exclusion handlers
  const handleAddExclusion = async (
    provider: ProviderType,
    pattern: string,
  ) => {
    await routerApi.addExclusion(provider, pattern);
    await refresh();
  };

  const handleRemoveExclusion = async (
    provider: ProviderType,
    pattern: string,
  ) => {
    await routerApi.removeExclusion(provider, pattern);
    await refresh();
  };

  // Injection handlers
  const handleToggleInjection = async (enabled: boolean) => {
    await injectionApi.setInjectionEnabled(enabled);
    setInjectionEnabled(enabled);
  };

  const handleAddInjectionRule = async (rule: InjectionRule) => {
    await injectionApi.addInjectionRule(rule);
    await refresh();
  };

  const handleRemoveInjectionRule = async (id: string) => {
    await injectionApi.removeInjectionRule(id);
    await refresh();
  };

  const handleUpdateInjectionRule = async (id: string, rule: InjectionRule) => {
    await injectionApi.updateInjectionRule(id, rule);
    await refresh();
  };

  const tabs: { id: TabType; label: string; count: number }[] = [
    { id: "aliases", label: "模型别名", count: aliases.length },
    { id: "rules", label: "路由规则", count: rules.length },
    {
      id: "exclusions",
      label: "排除列表",
      count: Object.values(exclusions).flat().length,
    },
    { id: "injection", label: "参数注入", count: injectionRules.length },
  ];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold flex items-center gap-2">
            <Route className="h-6 w-6" />
            智能路由
          </h2>
          <p className="text-muted-foreground">
            配置模型映射、路由规则和排除列表
          </p>
        </div>
        <button
          onClick={refresh}
          disabled={loading}
          className="flex items-center gap-2 rounded-lg border px-3 py-2 text-sm hover:bg-muted disabled:opacity-50"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          刷新
        </button>
      </div>

      <HelpTip title="智能路由说明" variant="blue">
        <ul className="list-disc list-inside space-y-1 text-sm text-blue-700 dark:text-blue-400">
          <li>
            <span className="font-medium">模型别名</span>
            ：使用熟悉的模型名（如 gpt-4）映射到实际模型
          </li>
          <li>
            <span className="font-medium">路由规则</span>
            ：将特定模型路由到指定 Provider，支持通配符匹配
          </li>
          <li>
            <span className="font-medium">排除列表</span>
            ：从特定 Provider 排除某些模型
          </li>
          <li>精确匹配规则优先于通配符规则</li>
        </ul>
      </HelpTip>

      {error && (
        <div className="rounded-lg border border-red-500 bg-red-50 p-4 text-red-700 dark:bg-red-950/30">
          {error}
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-2 border-b">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2 text-sm font-medium border-b-2 -mb-px flex items-center gap-2 ${
              activeTab === tab.id
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            {tab.label}
            {tab.count > 0 && (
              <span className="rounded-full bg-muted px-1.5 py-0.5 text-xs">
                {tab.count}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {loading ? (
        <div className="flex items-center justify-center py-12">
          <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <div className="py-4">
          {activeTab === "aliases" && (
            <ModelMapping
              aliases={aliases}
              onAdd={handleAddAlias}
              onRemove={handleRemoveAlias}
              loading={loading}
            />
          )}
          {activeTab === "rules" && (
            <RoutingRules
              rules={rules}
              onAdd={handleAddRule}
              onRemove={handleRemoveRule}
              onUpdate={handleUpdateRule}
              loading={loading}
            />
          )}
          {activeTab === "exclusions" && (
            <ExclusionList
              exclusions={exclusions}
              onAdd={handleAddExclusion}
              onRemove={handleRemoveExclusion}
              loading={loading}
            />
          )}
          {activeTab === "injection" && (
            <InjectionRules
              rules={injectionRules}
              enabled={injectionEnabled}
              onToggleEnabled={handleToggleInjection}
              onAdd={handleAddInjectionRule}
              onRemove={handleRemoveInjectionRule}
              onUpdate={handleUpdateInjectionRule}
              loading={loading}
            />
          )}
        </div>
      )}
    </div>
  );
});

RoutingPage.displayName = "RoutingPage";
