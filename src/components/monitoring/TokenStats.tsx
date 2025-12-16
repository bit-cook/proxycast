import React from "react";
import { Coins, TrendingUp, BarChart3 } from "lucide-react";
import type {
  TokenStatsSummary,
  ProviderTokenStats,
  PeriodTokenStats,
} from "@/lib/api/telemetry";

interface TokenStatsProps {
  summary: TokenStatsSummary;
  byProvider: Record<string, ProviderTokenStats>;
  byDay: PeriodTokenStats[];
}

export function TokenStats({ summary, byProvider, byDay }: TokenStatsProps) {
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

  const formatNumber = (num: number): string => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    return date.toLocaleDateString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
    });
  };

  // 计算每日最大值用于图表缩放
  const maxDayTokens = Math.max(...byDay.map((d) => d.total_tokens), 1);

  return (
    <div className="space-y-6">
      {/* Token 总览 */}
      <div className="grid grid-cols-4 gap-4">
        <TokenCard
          icon={Coins}
          label="总 Token"
          value={formatNumber(summary.total_tokens)}
          subValue={`${summary.record_count} 条记录`}
        />
        <TokenCard
          icon={TrendingUp}
          label="输入 Token"
          value={formatNumber(summary.total_input_tokens)}
          subValue={`平均 ${Math.round(summary.avg_input_tokens)}/请求`}
        />
        <TokenCard
          icon={TrendingUp}
          label="输出 Token"
          value={formatNumber(summary.total_output_tokens)}
          subValue={`平均 ${Math.round(summary.avg_output_tokens)}/请求`}
        />
        <TokenCard
          icon={BarChart3}
          label="数据来源"
          value={`${summary.actual_count}/${summary.record_count}`}
          subValue={`实际值 / 估算值 ${summary.estimated_count}`}
        />
      </div>

      {/* 按 Provider 统计 */}
      {Object.keys(byProvider).length > 0 && (
        <div className="rounded-lg border bg-card p-4">
          <h3 className="mb-4 font-semibold flex items-center gap-2">
            <Coins className="h-4 w-4" />按 Provider Token 使用
          </h3>
          <div className="space-y-3">
            {Object.entries(byProvider).map(([provider, stats]) => (
              <ProviderTokenRow
                key={provider}
                name={getProviderName(provider)}
                stats={stats}
                totalTokens={summary.total_tokens}
              />
            ))}
          </div>
        </div>
      )}

      {/* 每日趋势 */}
      {byDay.length > 0 && (
        <div className="rounded-lg border bg-card p-4">
          <h3 className="mb-4 font-semibold flex items-center gap-2">
            <BarChart3 className="h-4 w-4" />
            每日 Token 使用趋势
          </h3>
          <div className="flex items-end gap-1 h-32">
            {byDay
              .slice()
              .reverse()
              .map((day, index) => (
                <DayBar
                  key={index}
                  date={formatDate(day.period_start)}
                  tokens={day.total_tokens}
                  maxTokens={maxDayTokens}
                />
              ))}
          </div>
        </div>
      )}
    </div>
  );
}

function TokenCard({
  icon: Icon,
  label,
  value,
  subValue,
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  subValue?: string;
}) {
  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-center gap-2">
        <Icon className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm text-muted-foreground">{label}</span>
      </div>
      <div className="mt-2 text-2xl font-bold">{value}</div>
      {subValue && (
        <div className="mt-1 text-xs text-muted-foreground">{subValue}</div>
      )}
    </div>
  );
}

function ProviderTokenRow({
  name,
  stats,
  totalTokens,
}: {
  name: string;
  stats: ProviderTokenStats;
  totalTokens: number;
}) {
  const percentage =
    totalTokens > 0 ? (stats.total_tokens / totalTokens) * 100 : 0;

  const formatNumber = (num: number): string => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  return (
    <div className="flex items-center justify-between rounded-lg bg-muted/50 p-3">
      <div className="flex items-center gap-3">
        <div className="font-medium">{name}</div>
        <div className="text-sm text-muted-foreground">
          {stats.record_count} 条记录
        </div>
      </div>
      <div className="flex items-center gap-4">
        <div className="text-sm">
          <span className="text-muted-foreground">输入:</span>{" "}
          {formatNumber(stats.total_input_tokens)}
        </div>
        <div className="text-sm">
          <span className="text-muted-foreground">输出:</span>{" "}
          {formatNumber(stats.total_output_tokens)}
        </div>
        <div className="w-24">
          <div className="h-2 rounded-full bg-muted">
            <div
              className="h-2 rounded-full bg-primary"
              style={{ width: `${percentage}%` }}
            />
          </div>
        </div>
        <span className="text-sm font-medium w-16 text-right">
          {formatNumber(stats.total_tokens)}
        </span>
      </div>
    </div>
  );
}

function DayBar({
  date,
  tokens,
  maxTokens,
}: {
  date: string;
  tokens: number;
  maxTokens: number;
}) {
  const height = maxTokens > 0 ? (tokens / maxTokens) * 100 : 0;

  const formatNumber = (num: number): string => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(0)}K`;
    return num.toString();
  };

  return (
    <div className="flex-1 flex flex-col items-center gap-1">
      <div className="w-full flex items-end justify-center h-24">
        <div
          className="w-full max-w-8 bg-primary/80 rounded-t hover:bg-primary transition-colors"
          style={{ height: `${Math.max(height, 2)}%` }}
          title={`${formatNumber(tokens)} tokens`}
        />
      </div>
      <span className="text-xs text-muted-foreground">{date}</span>
    </div>
  );
}
