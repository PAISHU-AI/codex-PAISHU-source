import { BarChart3 } from "lucide-react";
import { formatTokens } from "../lib/format";
import type { DailyTokenBucket } from "../types/usage";

interface TrendPanelProps {
  buckets: DailyTokenBucket[];
  isLoading: boolean;
  sourceLabel: string;
  onOpenDetail: () => void;
}

export function TrendPanel({ buckets, isLoading, sourceLabel, onOpenDetail }: TrendPanelProps) {
  const max = Math.max(...buckets.map((bucket) => bucket.tokens), 1);
  return (
    <section className="panel trend-panel">
      <div className="section-heading">
        <div>
          <h2>7 日趋势</h2>
          <p>{sourceLabel}</p>
        </div>
        <button className="icon-button" onClick={onOpenDetail} aria-label="查看最近 30 天统计">
          <BarChart3 size={18} />
        </button>
      </div>
      {isLoading ? (
        <div className="skeleton block" />
      ) : buckets.length === 0 ? (
        <div className="empty-state">暂无令牌趋势。</div>
      ) : (
        <div className="trend-bars">
          {buckets.map((bucket) => (
            <div className="trend-item" key={bucket.id}>
              <div className="bar-shell">
                <span style={{ height: `${Math.max(6, (bucket.tokens / max) * 100)}%` }} />
              </div>
              <small>{bucket.label}</small>
              <em>{formatTokens(bucket.tokens)}</em>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
