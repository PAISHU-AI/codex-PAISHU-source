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
          <p>
            {isApiMode
              ? "API 模式使用本地会话统计"
              : "7 天官方剩余额度"}
          </p>
        </div>
        <Activity size={18} />
      </div>
      {isLoading ? (
        <div className="skeleton block" />
      ) : isApiMode ? (
        <div className="empty-state">API 模式不读取官方额度窗口，本地消耗见令牌价值与趋势。</div>
      ) : (
        <div className="quota-content">
          <QuotaRing window={snapshot?.secondary ?? null} />
          <SevenDayQuotaSummary window={snapshot?.secondary ?? null} />
        </div>
      )}
    </section>
  );
}

function QuotaRing({ window }: { window: RateWindow | null }) {
  const remaining = remainingPercent(window);
  const progress = remaining ?? 0;
  return (
    <div className="quota-ring" aria-label="7 天额度剩余百分比">
      <svg viewBox="0 0 160 160" role="img">
        <defs>
          <linearGradient id="sevenDayProgress" x1="12%" y1="0%" x2="88%" y2="100%">
            <stop offset="0%" stopColor="#7f8cff" />
            <stop offset="100%" stopColor="#bc8cff" />
          </linearGradient>
        </defs>
        <circle className="ring-track" cx="80" cy="80" r="58" />
        <circle
          className="ring-value"
          cx="80"
          cy="80"
          r="58"
          pathLength="100"
          strokeDasharray={`${progress} 100`}
        />
      </svg>
      <div className="ring-label">
        <strong>{formatPercent(remaining)}</strong>
        <span>{window ? "7天剩余" : "暂无额度"}</span>
      </div>
    </div>
  );
}

function SevenDayQuotaSummary({ window }: { window: RateWindow | null }) {
  const remaining = remainingPercent(window);
  const used = window?.usedPercent ?? null;
  return (
    <div className="quota-summary">
      <div className="quota-summary-header">
        <span className="quota-source-badge">官方 7 天额度</span>
        <span className="quota-window-label">7 天滚动窗口</span>
      </div>
      {window ? (
        <>
          <div className="quota-summary-main">
            <div>
              <span>可用额度</span>
              <strong>{formatPercent(remaining)}</strong>
            </div>
            <span className="quota-used">已用 {used}%</span>
          </div>
          <div className="quota-meter" aria-label={`已用 ${used}%`}>
            <span style={{ width: `${used ?? 0}%` }} />
          </div>
          <div className="quota-reset">
            <span className="quota-reset-icon">
              <Clock size={14} />
            </span>
            <div>
              <span>窗口重置</span>
              <strong>{formatTime(window.resetsAt)}</strong>
            </div>
          </div>
        </>
      ) : (
        <div className="quota-unavailable">
          <Clock size={12} />
          官方当前未返回 7 天额度窗口
        </div>
      )}
    </div>
  );
}
