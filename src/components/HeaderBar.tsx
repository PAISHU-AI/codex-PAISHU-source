import { Minus, RefreshCw, Settings, ShieldCheck, X } from "lucide-react";
import type { PointerEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { formatTime } from "../lib/format";
import appLogo from "../assets/paishu-agi-logo.png";
import type { UsageSnapshot } from "../types/usage";

interface HeaderBarProps {
  snapshot: UsageSnapshot | null;
  isRefreshing: boolean;
  onRefresh: () => void;
  onOpenSettings: () => void;
}

export function HeaderBar({ snapshot, isRefreshing, onRefresh, onOpenSettings }: HeaderBarProps) {
  const plan = formatPlan(snapshot?.account?.planType ?? snapshot?.account?.accountType);
  return (
    <header className="header-bar" data-tauri-drag-region onPointerDown={startDragFromHeader}>
      <div className="brand-lockup">
        <div className="brand-mark" aria-hidden="true">
          <img src={appLogo} alt="" draggable={false} />
        </div>
        <div>
          <h1>光核超级服务</h1>
          <p>Codex 额度、令牌与任务遥测</p>
        </div>
      </div>

      <div className="header-status" aria-label="应用状态">
        <span className="status-pill">
          <ShieldCheck size={14} />
          {plan}
        </span>
        <span className="muted">上次同步 {formatTime(snapshot?.refreshedAt)}</span>
        <button className="icon-button" onClick={onRefresh} aria-label="刷新使用快照">
          <RefreshCw size={16} className={isRefreshing ? "spin" : ""} />
        </button>
        <button className="icon-button" onClick={onOpenSettings} aria-label="打开设置">
          <Settings size={16} />
        </button>
        <div className="window-controls" aria-label="窗口控制">
          <button
            className="icon-button window-control"
            onClick={minimizeWindow}
            aria-label="最小化窗口"
          >
            <Minus size={15} />
          </button>
          <button
            className="icon-button window-control close"
            onClick={closeWindow}
            title="隐藏到顶部菜单栏"
            aria-label="隐藏到顶部菜单栏"
          >
            <X size={15} />
          </button>
        </div>
      </div>
    </header>
  );
}

function startDragFromHeader(event: PointerEvent<HTMLElement>) {
  if (event.button !== 0) return;
  const target = event.target;
  if (!(target instanceof HTMLElement)) return;
  if (target.closest("button, a, input, select, textarea")) return;
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;

  void getCurrentWindow()
    .startDragging()
    .catch(() => undefined);
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function minimizeWindow() {
  if (!isTauriRuntime()) return;
  void getCurrentWindow()
    .minimize()
    .catch(() => undefined);
}

function closeWindow() {
  if (!isTauriRuntime()) return;
  void getCurrentWindow()
    .close()
    .catch(() => undefined);
}

function formatPlan(plan?: string | null): string {
  if (!plan) return "离线";
  const normalized = plan.toLowerCase();
  if (normalized === "chatgpt") return "ChatGPT";
  if (normalized === "pro") return "Pro 计划";
  if (normalized === "plus") return "Plus 计划";
  return `${plan} 计划`;
}
