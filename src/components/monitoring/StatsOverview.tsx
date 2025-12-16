import React from "react";
import {
  Activity,
  CheckCircle2,
  XCircle,
  Clock,
  Zap,
  TrendingUp,
} from "lucide-react";
import type { StatsSummary, ProviderStats } from "@/lib/api/telemetry";

interface StatsOverviewProps {
  stats: StatsSummary;
  byProvider: Record<string, ProviderStats>;
}

export function StatsOverview({ stats, byProvider }: StatsOverviewProps) {
  const formatLatency = (ms: number) => {
    if (ms < 1000) return `${Math.round(ms)}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const formatRate = (rate: number) => `${(rate * 100).toFixed(1)}%`;

  const getProviderName = (id: string) => {
    const names: Record<string, string> = {
      kiro: "Kiro Claude",
      gemini: "Gemini",
      qwen: "通义千问",
      openai: "OpenAI",
      claude: "Claude",
      antigravity: "Antigravity",
    };
    return names[id] || id;
  };

  return (
    <div className="space-y-6">
      {/* 总体统计卡片 */}
      <div className="grid grid-cols-4 gap-4">
        <StatCard
          icon={Activity}
          label="总请求数"
          value={stats.total_requests.toString()}
          subValue={`成功 ${stats.successful_requests}`}
        />
        <StatCard
          icon={CheckCircle2}
          label="成功率"
          value={formatRate(stats.success_rate)}
          subValue={`失败 ${stats.failed_requests}`}
          valueColor={
            stats.success_rate >= 0.9
              ? "text-green-600"
              : stats.success_rate >= 0.7
                ? "text-yellow-600"
                : "text-red-600"
          }
        />
        <StatCard
          icon={Clock}
          label="平均延迟"
          value={formatLatency(stats.avg_latency_ms)}
          subValue={
            stats.max_latency_ms
              ? `最大 ${formatLatency(stats.max_latency_ms)}`
              : undefined
          }
        />
        <StatCard
          icon={Zap}
          label="总 Token"
          value={formatNumber(stats.total_tokens)}
          subValue={`输入 ${formatNumber(stats.total_input_tokens)} / 输出 ${formatNumber(stats.total_output_tokens)}`}
        />
      </div>

      {/* 按 Provider 统计 */}
      {Object.keys(byProvider).length > 0 && (
        <div className="rounded-lg border bg-card p-4">
          <h3 className="mb-4 font-semibold flex items-center gap-2">
            <TrendingUp className="h-4 w-4" />按 Provider 统计
          </h3>
          <div className="space-y-3">
            {Object.entries(byProvider).map(([provider, providerStats]) => (
              <ProviderStatRow
                key={provider}
                name={getProviderName(provider)}
                stats={providerStats}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  subValue,
  valueColor = "text-foreground",
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  subValue?: string;
  valueColor?: string;
}) {
  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-center gap-2">
        <Icon className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm text-muted-foreground">{label}</span>
      </div>
      <div className={`mt-2 text-2xl font-bold ${valueColor}`}>{value}</div>
      {subValue && (
        <div className="mt-1 text-xs text-muted-foreground">{subValue}</div>
      )}
    </div>
  );
}

function ProviderStatRow({
  name,
  stats,
}: {
  name: string;
  stats: ProviderStats;
}) {
  const successRate = stats.success_rate * 100;

  return (
    <div className="flex items-center justify-between rounded-lg bg-muted/50 p-3">
      <div className="flex items-center gap-3">
        <div className="font-medium">{name}</div>
        <div className="text-sm text-muted-foreground">
          {stats.total_requests} 请求
        </div>
      </div>
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-1">
          <CheckCircle2 className="h-3 w-3 text-green-500" />
          <span className="text-sm">{stats.successful_requests}</span>
        </div>
        <div className="flex items-center gap-1">
          <XCircle className="h-3 w-3 text-red-500" />
          <span className="text-sm">{stats.failed_requests}</span>
        </div>
        <div className="w-20">
          <div className="h-2 rounded-full bg-muted">
            <div
              className={`h-2 rounded-full ${
                successRate >= 90
                  ? "bg-green-500"
                  : successRate >= 70
                    ? "bg-yellow-500"
                    : "bg-red-500"
              }`}
              style={{ width: `${successRate}%` }}
            />
          </div>
        </div>
        <span className="text-sm font-medium w-12 text-right">
          {successRate.toFixed(0)}%
        </span>
      </div>
    </div>
  );
}

function formatNumber(num: number): string {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
}
