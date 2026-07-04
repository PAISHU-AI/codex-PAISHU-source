import type { RateWindow, TokenBreakdown } from "../types/usage";

export function formatTokens(value?: number | null): string {
  if (value == null) return "--";
  const abs = Math.abs(value);
  const formatter = new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 1 });
  if (abs >= 100_000_000) return `${formatter.format(value / 100_000_000)}亿`;
  if (abs >= 10_000) return `${formatter.format(value / 10_000)}万`;
  if (abs >= 1_000) return `${formatter.format(value / 1_000)}千`;
  return new Intl.NumberFormat("zh-CN").format(value);
}

export function formatUsd(value?: number | null): string {
  if (value == null) return "--";
  const digits = Math.abs(value) >= 1000 ? 0 : 2;
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "USD",
    currencyDisplay: "narrowSymbol",
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(value);
}

export function formatPercent(value?: number | null): string {
  if (value == null || Number.isNaN(value)) return "--";
  if (value > 0 && value < 1) return "<1%";
  return `${Math.round(value)}%`;
}

export function remainingPercent(window?: RateWindow | null): number | null {
  if (!window) return null;
  return Math.max(0, Math.min(100, 100 - window.usedPercent));
}

export function formatTime(epoch?: number | null): string {
  if (!epoch) return "--";
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(epoch * 1000));
}

export function splitTokens(tokens?: TokenBreakdown | null) {
  const input = Math.max(tokens?.inputTokens ?? 0, 0);
  const cached = Math.min(Math.max(tokens?.cachedInputTokens ?? 0, 0), input);
  const uncached = Math.max(input - cached, 0);
  const output = Math.max(tokens?.outputTokens ?? 0, 0);
  if (tokens && input + cached + output === 0 && tokens.totalTokens > 0) {
    return { uncached: tokens.totalTokens, cached: 0, output: 0, total: tokens.totalTokens };
  }
  const total = Math.max(uncached + cached + output, 1);
  return { uncached, cached, output, total };
}
