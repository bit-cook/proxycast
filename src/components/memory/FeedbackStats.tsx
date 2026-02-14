import { useEffect, useState } from 'react';
import { getFeedbackStats, type FeedbackStats as FeedbackStatsData } from '@/lib/api/memoryFeedback';

interface FeedbackStatsProps {
    sessionId: string;
}

export function FeedbackStats({ sessionId }: FeedbackStatsProps) {
    const [stats, setStats] = useState<FeedbackStatsData | null>(null);

    useEffect(() => {
        let cancelled = false;

        if (!sessionId) {
            setStats(null);
            return;
        }

        getFeedbackStats(sessionId)
            .then((result) => {
                if (!cancelled) {
                    setStats(result);
                }
            })
            .catch(() => {
                if (!cancelled) {
                    setStats(null);
                }
            });

        return () => {
            cancelled = true;
        };
    }, [sessionId]);

    if (!stats) return null;

    return (
        <div className="grid grid-cols-4 gap-4">
            <StatCard label="总数" value={stats.total} />
            <StatCard label="批准" value={stats.approve_count} />
            <StatCard label="拒绝" value={stats.reject_count} />
            <StatCard label="批准率" value={`${(stats.approval_rate * 100).toFixed(1)}%`} />
        </div>
    );
}

function StatCard({ label, value }: { label: string; value: number | string }) {
    return (
        <div className="rounded-lg border p-4">
            <div className="text-sm text-muted-foreground">{label}</div>
            <div className="text-2xl font-bold">{value}</div>
        </div>
    );
}
