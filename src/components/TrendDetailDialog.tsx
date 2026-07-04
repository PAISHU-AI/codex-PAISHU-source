import { X } from "lucide-react";
import { formatTokens } from "../lib/format";
import type { DailyTokenBucket } from "../types/usage";

interface TrendDetailDialogProps {
  buckets: DailyTokenBucket[];
  sourceLabel: string;
  onClose: () => void;
}

export function TrendDetailDialog({ buckets, sourceLabel, onClose }: TrendDetailDialogProps) {
  const max = Math.max(...buckets.map((bucket) => bucket.tokens), 1);

  return (
    <div className="modal-backdrop" role="presentation" onClick={onClose}>
      <section
        className="trend-detail-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="trend-detail-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="dialog-header">
          <div>
            <h2 id="trend-detail-title">最近 30 天用量统计</h2>
            <p>{sourceLabel}</p>
          </div>
          <button className="icon-button" onClick={onClose} aria-label="关闭 30 天统计">
            <X size={18} />
          </button>
        </div>

        {buckets.length === 0 ? (
          <div className="empty-state">暂无最近 30 天用量。</div>
        ) : (
          <div className="trend-detail-list">
            {buckets.map((bucket) => {
              const width = Math.max(2, (bucket.tokens / max) * 100);
              return (
                <div className="trend-detail-row" key={bucket.id}>
                  <span className="trend-detail-date">{bucket.label}</span>
                  <div className="trend-detail-meter" aria-hidden="true">
                    <span style={{ width: `${width}%` }} />
                  </div>
                  <strong>{formatTokens(bucket.tokens)}</strong>
                </div>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}
