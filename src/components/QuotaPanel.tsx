import { Activity, Clock } from "lucide-react";
import { formatPercent, formatTime, remainingPercent } from "../lib/format";
import type { CodexAccessMode, RateWindow, UsageSnapshot } from "../types/usage";

interface QuotaPanelProps {
  snapshot: UsageSnapshot | null;
  isLoading: boolean;
  accessMode?: CodexAccessMode;
}

export function QuotaPanel({ snapshot, isLoading, accessMode = "official" }: QuotaPanelProps) {
  const isApiMode = accessMode === "relay";
  return (
    <section className="panel quota-panel">
      <div className="section-heading">
        <div>
          <h2>额度窗口</h2>
          <p>{isApiMode ? "API 模式使用本地会话统计" : "5 小时与 7 天滚动使用窗口"}</p>
        </div>
        <Activity size={18} />
      </div>
      {isLoading ? (
        <div className="skeleton block" />
      ) : isApiMode ? (
        <div className="empty-state">API 模式不读取官方额度窗口，本地消耗见令牌价值与趋势。</div>
      ) : (
        <div className="quota-content">
          <DualRing primary={snapshot?.primary ?? null} secondary={snapshot?.secondary ?? null} />
          <div className="quota-lines">
            <QuotaLine label="5 小时剩余" window={snapshot?.primary ?? null} accent="primary" />
            <QuotaLine label="7 天剩余" window={snapshot?.secondary ?? null} accent="secondary" />
          </div>
        </div>
      )}
    </section>
  );
}

function DualRing({
  primary,
  secondary,
}: {
  primary: RateWindow | null;
  secondary: RateWindow | null;
}) {
  const primaryValue = remainingPercent(primary) ?? 0;
  const secondaryValue = remainingPercent(secondary) ?? 0;
  return (
    <div className="dual-ring" aria-label="额度剩余百分比">
      <svg viewBox="0 0 160 160" role="img">
        <circle className="ring-track outer" cx="80" cy="80" r="66" />
        <circle
          className="ring-value outer"
          cx="80"
          cy="80"
          r="66"
          pathLength="100"
          strokeDasharray={`${primaryValue} 100`}
        />
        <circle className="ring-track inner" cx="80" cy="80" r="48" />
        <circle
          className="ring-value inner"
          cx="80"
          cy="80"
          r="48"
          pathLength="100"
          strokeDasharray={`${secondaryValue} 100`}
        />
      </svg>
      <div className="ring-label">
        <strong>{formatPercent(primaryValue)}</strong>
        <span>剩余</span>
      </div>
    </div>
  );
}

function QuotaLine({
  label,
  window,
  accent,
}: {
  label: string;
  window: RateWindow | null;
  accent: "primary" | "secondary";
}) {
  const remaining = remainingPercent(window);
  return (
    <div className={`quota-line ${accent}`}>
      <div>
        <span>{label}</span>
        <strong>{formatPercent(remaining)}</strong>
      </div>
      <small>
        <Clock size={12} />
        重置于 {formatTime(window?.resetsAt)}
      </small>
    </div>
  );
}
