import { Banknote, Cpu, Database, Send } from "lucide-react";
import type { ReactNode } from "react";
import { formatTokens, formatUsd, splitTokens } from "../lib/format";
import type { DetailedUsage, PricedTokenUsage } from "../types/usage";

interface TokenValuePanelProps {
  usage: DetailedUsage | null;
  valuePeriodUsage?: PricedTokenUsage | null;
  isLoading: boolean;
  sourceLabel: string;
}

export function TokenValuePanel({
  usage,
  valuePeriodUsage,
  isLoading,
  sourceLabel,
}: TokenValuePanelProps) {
  return (
    <section className="panel token-panel">
      <div className="section-heading">
        <div>
          <h2>令牌价值</h2>
          <p>{sourceLabel}，按官方输入/缓存/输出价格估算</p>
        </div>
        <Banknote size={18} />
      </div>
      {isLoading ? (
        <div className="skeleton block" />
      ) : (
        <>
          <div className="metric-row">
            <Metric label="今日" usage={usage?.today ?? null} icon={<Cpu size={15} />} />
            <Metric label="近 7 天" usage={usage?.sevenDay ?? null} icon={<Database size={15} />} />
            <Metric label="累计" usage={usage?.lifetime ?? null} icon={<Send size={15} />} />
          </div>
          <ValueEstimate usage={valuePeriodUsage ?? usage?.month ?? null} />
        </>
      )}
    </section>
  );
}

function Metric({
  label,
  usage,
  icon,
}: {
  label: string;
  usage: PricedTokenUsage | null;
  icon: ReactNode;
}) {
  const split = splitTokens(usage?.tokens);
  return (
    <div className="metric-card">
      <div className="metric-title">
        {icon}
        <span>{label}</span>
      </div>
      <strong>{formatTokens(usage?.tokens.totalTokens)}</strong>
      <small>{formatUsd(usage?.estimatedCostUsd)}</small>
      <div className="split-bar" aria-label={`${label}令牌构成`}>
        <span className="uncached" style={{ width: `${(split.uncached / split.total) * 100}%` }} />
        <span className="cached" style={{ width: `${(split.cached / split.total) * 100}%` }} />
        <span className="output" style={{ width: `${(split.output / split.total) * 100}%` }} />
      </div>
    </div>
  );
}

function ValueEstimate({ usage }: { usage: PricedTokenUsage | null }) {
  const value = usage?.estimatedCostUsd ?? 0;
  return (
    <div className="value-estimate">
      <div>
        <span>本期价值估算</span>
        <small>按当前会员周期内的输入、缓存输入、输出令牌估算</small>
      </div>
      <strong>{formatUsd(value)}</strong>
    </div>
  );
}
