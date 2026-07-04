import { selectDashboardData } from "./dashboardData";
import { mockSnapshot } from "./mock";

describe("selectDashboardData", () => {
  it("uses official account data in official mode", () => {
    const selection = selectDashboardData(mockSnapshot, "official");

    expect(selection.tokenUsage).toBe(mockSnapshot.officialUsage?.detailedUsage);
    expect(selection.valuePeriodUsage).toBe(mockSnapshot.officialUsage?.valuePeriod);
    expect(selection.trendBuckets).toBe(mockSnapshot.officialUsage?.dailyBuckets);
    expect(selection.trendSourceLabel).toBe("官方账户每日令牌");
  });

  it("uses local data in API relay mode even when official data exists", () => {
    const selection = selectDashboardData(mockSnapshot, "relay");

    expect(selection.tokenUsage).toBe(mockSnapshot.local?.detailedUsage);
    expect(selection.valuePeriodUsage).toBe(mockSnapshot.local?.detailedUsage?.valuePeriod);
    expect(selection.trendBuckets).toBe(mockSnapshot.local?.dailyBuckets);
    expect(selection.trendSourceLabel).toBe("本机会话令牌合计");
  });

  it("falls back to local data in official mode when official usage is unavailable", () => {
    const snapshot = { ...mockSnapshot, officialUsage: null };
    const selection = selectDashboardData(snapshot, "official");

    expect(selection.tokenUsage).toBe(mockSnapshot.local?.detailedUsage);
    expect(selection.valuePeriodUsage).toBe(mockSnapshot.local?.detailedUsage?.valuePeriod);
    expect(selection.trendBuckets).toBe(mockSnapshot.local?.dailyBuckets);
    expect(selection.trendSourceLabel).toBe("官方暂不可用，显示本机会话令牌");
  });

  it("uses local real-time today usage when official daily usage has not caught up", () => {
    const officialUsage = {
      ...mockSnapshot.officialUsage!,
      detailedUsage: {
        ...mockSnapshot.officialUsage!.detailedUsage,
        today: {
          estimatedCostUsd: 0,
          tokens: {
            inputTokens: 0,
            cachedInputTokens: 0,
            outputTokens: 0,
            reasoningOutputTokens: 0,
            totalTokens: 0,
          },
        },
      },
      dailyBuckets: mockSnapshot.officialUsage!.dailyBuckets.map((bucket, index, buckets) =>
        index === buckets.length - 1 ? { ...bucket, tokens: 0 } : bucket,
      ),
    };
    const local = {
      ...mockSnapshot.local!,
      dailyBuckets: mockSnapshot.local!.dailyBuckets.map((bucket, index, buckets) =>
        index === buckets.length - 1 ? { ...bucket, tokens: 1_600_000_000 } : bucket,
      ),
    };
    const snapshot = { ...mockSnapshot, officialUsage, local };
    const selection = selectDashboardData(snapshot, "official");

    expect(selection.tokenUsage?.today).toBe(mockSnapshot.local?.detailedUsage?.today);
    expect(selection.trendBuckets.at(-1)?.tokens).toBe(
      mockSnapshot.local?.detailedUsage?.today.tokens.totalTokens,
    );
    expect(selection.trendBuckets.at(-1)?.tokens).not.toBe(local.dailyBuckets.at(-1)?.tokens);
    expect(selection.tokenSourceLabel).toBe("官方账户每日用量，今日使用本地实时补充");
  });
});
