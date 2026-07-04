import type {
  CodexAccessMode,
  DailyTokenBucket,
  DetailedUsage,
  PricedTokenUsage,
  UsageSnapshot,
} from "../types/usage";

export interface DashboardDataSelection {
  tokenUsage: DetailedUsage | null;
  valuePeriodUsage: PricedTokenUsage | null;
  trendBuckets: DailyTokenBucket[];
  trendDetailBuckets: DailyTokenBucket[];
  trendSourceLabel: string;
  tokenSourceLabel: string;
}

export function selectDashboardData(
  snapshot: UsageSnapshot | null,
  accessMode: CodexAccessMode,
): DashboardDataSelection {
  const isApiMode = accessMode === "relay";
  const officialUsage = isApiMode ? null : (snapshot?.officialUsage ?? null);
  const localUsage = snapshot?.local ?? null;

  if (isApiMode) {
    const trendBuckets = localUsage?.dailyBuckets ?? [];
    return {
      tokenUsage: localUsage?.detailedUsage ?? null,
      valuePeriodUsage: localUsage?.detailedUsage?.valuePeriod ?? null,
      trendBuckets,
      trendDetailBuckets: trendBuckets,
      trendSourceLabel: "本机会话令牌合计",
      tokenSourceLabel: "本机会话精细用量",
    };
  }

  const fallbackToLocal = !officialUsage && localUsage;
  const trendBuckets = fallbackToLocal
    ? (localUsage.dailyBuckets ?? [])
    : withLocalTodayBucket(
        officialUsage?.dailyBuckets ?? [],
        localUsage?.detailedUsage?.today ?? null,
      );
  const tokenUsage = fallbackToLocal
    ? (localUsage.detailedUsage ?? null)
    : withLocalTodayUsage(officialUsage?.detailedUsage ?? null, localUsage?.detailedUsage ?? null);
  const usesLocalToday =
    Boolean(officialUsage) &&
    isZeroUsage(officialUsage?.detailedUsage?.today ?? null) &&
    hasUsage(localUsage?.detailedUsage?.today ?? null);
  return {
    tokenUsage,
    valuePeriodUsage: fallbackToLocal
      ? (localUsage.detailedUsage?.valuePeriod ?? null)
      : (officialUsage?.valuePeriod ?? null),
    trendBuckets,
    trendDetailBuckets: fallbackToLocal ? trendBuckets : (officialUsage?.recentDailyBuckets ?? trendBuckets),
    trendSourceLabel: fallbackToLocal
      ? "官方暂不可用，显示本机会话令牌"
      : usesLocalToday
        ? "官方账户每日令牌，今日使用本地实时补充"
        : "官方账户每日令牌",
    tokenSourceLabel: fallbackToLocal
      ? "官方暂不可用，显示本机会话精细用量"
      : usesLocalToday
        ? "官方账户每日用量，今日使用本地实时补充"
        : "官方账户每日用量",
  };
}

function withLocalTodayUsage(
  official: DetailedUsage | null,
  local: DetailedUsage | null,
): DetailedUsage | null {
  if (!official) return null;
  if (!isZeroUsage(official.today) || !hasUsage(local?.today ?? null)) return official;
  return { ...official, today: local!.today };
}

function withLocalTodayBucket(
  officialBuckets: DailyTokenBucket[],
  localTodayUsage: PricedTokenUsage | null,
): DailyTokenBucket[] {
  const officialToday = officialBuckets.at(-1);
  const localTodayTokens = localTodayUsage?.tokens.totalTokens ?? 0;
  if (!officialToday || officialToday.tokens > 0 || localTodayTokens <= 0) {
    return officialBuckets;
  }
  return [...officialBuckets.slice(0, -1), { ...officialToday, tokens: localTodayTokens }];
}

function isZeroUsage(usage: PricedTokenUsage | null): boolean {
  return !hasUsage(usage);
}

function hasUsage(usage: PricedTokenUsage | null): boolean {
  return (usage?.tokens.totalTokens ?? 0) > 0;
}
