import { Brain, Gauge, KeyRound, ShieldCheck, Server } from "lucide-react";
import type { ReactNode } from "react";
import type { ApiSpeedMode, AppSettings, ReasoningEffort, UsageSnapshot } from "../types/usage";

interface LoginStatusCardProps {
  snapshot: UsageSnapshot | null;
  settings: AppSettings;
}

export function LoginStatusCard({ snapshot, settings }: LoginStatusCardProps) {
  const isRelay = settings.accessMode === "relay";
  const accountReady = Boolean(snapshot?.account?.emailPresent || snapshot?.authStatus?.isLoggedIn);
  const relayReady = Boolean(settings.apiEndpoint?.trim());
  const statusTone = isRelay
    ? relayReady
      ? "relay"
      : "warning"
    : accountReady
      ? "official"
      : "warning";
  const title = isRelay ? "API 中转模式" : accountReady ? "官方原生已登录" : "官方原生待确认";
  const detail = isRelay
    ? relayReady
      ? "中转端点已配置，本地统计已启用"
      : "需要配置 API 端点"
    : accountReady
      ? snapshot?.account?.planType
        ? `${snapshot.account.planType} 账户可用`
        : "已检测到本机 ChatGPT 登录凭据"
      : "未读取到官方账户状态";
  const effectiveState = isRelay ? "API 模式使用本地 SQLite/JSONL 统计" : "官方监控已生效";

  return (
    <section className={`panel login-panel ${statusTone}`}>
      <div className="section-heading">
        <div>
          <h2>登录状态</h2>
          <p>当前 Codex 接入方式</p>
        </div>
      </div>

      <div className="login-state">
        <div className="login-state-icon">
          {isRelay ? <Server size={20} /> : <ShieldCheck size={20} />}
        </div>
        <div>
          <strong>{title}</strong>
          <span>{detail}</span>
        </div>
      </div>

      <div className="login-grid" aria-label="Codex 接入配置摘要">
        <StatusField
          icon={<Server size={14} />}
          label="端点"
          value={isRelay ? compactEndpoint(settings.apiEndpoint) : "官方默认"}
        />
        <StatusField
          icon={<KeyRound size={14} />}
          label="模型"
          value={isRelay ? settings.apiModel || "gpt-5" : "官方默认"}
        />
        <StatusField
          icon={<Brain size={14} />}
          label="推理"
          value={isRelay ? reasoningLabel(settings.reasoningEffort) : "官方默认"}
        />
        <StatusField
          icon={<Gauge size={14} />}
          label="速度策略"
          value={isRelay ? speedLabel(settings.speedMode) : "官方默认"}
        />
      </div>
      <p className="login-effect">{effectiveState}</p>
    </section>
  );
}

function StatusField({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div className="login-field">
      <span>
        {icon}
        {label}
      </span>
      <strong title={value}>{value}</strong>
    </div>
  );
}

function compactEndpoint(value?: string | null): string {
  if (!value) return "未配置";
  try {
    return new URL(value).host;
  } catch {
    return value;
  }
}

function reasoningLabel(value: ReasoningEffort): string {
  const labels: Record<ReasoningEffort, string> = {
    minimal: "极低",
    low: "低",
    medium: "中",
    high: "高",
    extreme: "超高",
  };
  return labels[value];
}

function speedLabel(value: ApiSpeedMode): string {
  const labels: Record<ApiSpeedMode, string> = {
    stable: "稳定",
    balanced: "均衡",
    fast: "快速",
  };
  return labels[value];
}
