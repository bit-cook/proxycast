import React, { useState, useEffect, useCallback } from "react";
import { Activity, FileText, Coins, RefreshCw } from "lucide-react";
import { StatsOverview } from "./StatsOverview";
import { LogViewer } from "./LogViewer";
import { TokenStats } from "./TokenStats";
import {
  getDashboardData,
  getRequestLogs,
  clearRequestLogs,
  getTokenStatsByDay,
  getTokenStatsByProvider,
  type DashboardData,
  type RequestLog,
  type PeriodTokenStats,
  type ProviderTokenStats,
  type RequestStatus,
} from "@/lib/api/telemetry";

type TabType = "overview" | "logs" | "tokens";

export function MonitoringPage() {
  const [activeTab, setActiveTab] = useState<TabType>("overview");
  const [dashboardData, setDashboardData] = useState<DashboardData | null>(
    null,
  );
  const [logs, setLogs] = useState<RequestLog[]>([]);
  const [tokensByDay, setTokensByDay] = useState<PeriodTokenStats[]>([]);
  const [tokensByProvider, setTokensByProvider] = useState<
    Record<string, ProviderTokenStats>
  >({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      const [dashboard, logsData, dayStats, providerStats] = await Promise.all([
        getDashboardData(),
        getRequestLogs({ limit: 100 }),
        getTokenStatsByDay(7),
        getTokenStatsByProvider({ preset: "7d" }),
      ]);

      setDashboardData(dashboard);
      setLogs(logsData);
      setTokensByDay(dayStats);
      setTokensByProvider(providerStats);
    } catch (e) {
      console.error("Failed to fetch monitoring data:", e);
      setError(e instanceof Error ? e.message : "加载数据失败");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
    // 每 30 秒刷新一次
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, [fetchData]);

  const handleClearLogs = async () => {
    try {
      await clearRequestLogs();
      setLogs([]);
    } catch (e) {
      console.error("Failed to clear logs:", e);
    }
  };

  const handleFilterLogs = async (filter: {
    provider?: string;
    status?: RequestStatus;
  }) => {
    try {
      const filteredLogs = await getRequestLogs({
        ...filter,
        limit: 100,
      });
      setLogs(filteredLogs);
    } catch (e) {
      console.error("Failed to filter logs:", e);
    }
  };

  const tabs: { id: TabType; label: string; icon: React.ElementType }[] = [
    { id: "overview", label: "统计概览", icon: Activity },
    { id: "logs", label: "请求日志", icon: FileText },
    { id: "tokens", label: "Token 统计", icon: Coins },
  ];

  return (
    <div className="space-y-6">
      {/* 页面标题 */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">监控中心</h2>
          <p className="text-muted-foreground">
            请求统计、日志和 Token 使用情况
          </p>
        </div>
        <button
          onClick={fetchData}
          disabled={loading}
          className="flex items-center gap-2 rounded-lg border px-3 py-2 text-sm hover:bg-muted disabled:opacity-50"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          刷新
        </button>
      </div>

      {/* 标签页 */}
      <div className="flex gap-1 border-b">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            <tab.icon className="h-4 w-4" />
            {tab.label}
          </button>
        ))}
      </div>

      {/* 错误提示 */}
      {error && (
        <div className="rounded-lg border border-red-200 bg-red-50 dark:bg-red-950/20 p-4 text-red-600 dark:text-red-400">
          {error}
        </div>
      )}

      {/* 内容区域 */}
      {loading && !dashboardData ? (
        <div className="flex items-center justify-center py-12">
          <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <>
          {activeTab === "overview" && dashboardData && (
            <StatsOverview
              stats={dashboardData.stats}
              byProvider={dashboardData.by_provider}
            />
          )}

          {activeTab === "logs" && (
            <LogViewer
              logs={logs}
              onClear={handleClearLogs}
              onFilter={handleFilterLogs}
            />
          )}

          {activeTab === "tokens" && dashboardData && (
            <TokenStats
              summary={dashboardData.tokens}
              byProvider={tokensByProvider}
              byDay={tokensByDay}
            />
          )}
        </>
      )}
    </div>
  );
}
