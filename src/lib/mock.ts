import type { AppSettings, DailyTokenBucket, UsageSnapshot } from "../types/usage";

const now = Math.floor(Date.now() / 1000);
const mockTrendTokens = [
  4_200_000, 6_500_000, 3_100_000, 7_800_000, 9_900_000, 5_300_000, 11_200_000, 8_600_000,
  13_100_000, 4_900_000, 10_400_000, 6_700_000, 15_300_000, 7_400_000, 9_100_000, 12_600_000,
  5_700_000, 14_800_000, 16_900_000, 10_200_000, 7_015_000, 13_000_000, 6_033_000, 6_200_000,
  9_400_000, 12_100_000, 8_700_000, 21_200_000, 16_500_000, 18_240_000,
];
const mockRecentDailyBuckets: DailyTokenBucket[] = mockTrendTokens.map((tokens, index) => {
  const date = new Date();
  date.setDate(date.getDate() - (mockTrendTokens.length - 1 - index));
  return {
    id: date.toISOString().slice(0, 10),
    label: `${date.getMonth() + 1}/${date.getDate()}`,
    tokens,
  };
});
const mockDailyBuckets = mockRecentDailyBuckets.slice(-7);

export const defaultSettings: AppSettings = {
  language: "zh",
  theme: "system",
  alwaysOnTop: false,
  showOnStart: true,
  codexBinaryPath: null,
  codexDataDir: null,
  refreshIntervalSecs: 300,
  showTaskBoard: true,
  accessMode: "official",
  apiEndpoint: null,
  apiKey: null,
  apiModel: "gpt-5",
  reasoningEffort: "medium",
  speedMode: "balanced",
  membershipStartedOn: null,
};

export const mockSnapshot: UsageSnapshot = {
  refreshedAt: now,
  account: { accountType: "chatgpt", planType: "Pro", emailPresent: true },
  limitId: "codex",
  limitName: "Codex",
  primary: { usedPercent: 37, windowDurationMins: 300, resetsAt: now + 7200 },
  secondary: { usedPercent: 58, windowDurationMins: 10080, resetsAt: now + 172800 },
  credits: { hasCredits: true, unlimited: false, balance: "active", resetCredits: 12 },
  cloudLifetimeTokens: 785_000_000,
  officialUsage: {
    lifetimeTokens: 785_000_000,
    todayTokens: 18_240_000,
    sevenDayTokens: 92_300_000,
    monthTokens: 246_000_000,
    valuePeriodStart: "2026-06-27",
    valuePeriodSource: "officialRecord",
    valuePeriodTokens: 92_300_000,
    valuePeriod: {
      estimatedCostUsd: 115.38,
      tokens: {
        inputTokens: 92_300_000,
        cachedInputTokens: 0,
        outputTokens: 0,
        reasoningOutputTokens: 0,
        totalTokens: 92_300_000,
      },
    },
    dailyBuckets: mockDailyBuckets,
    recentDailyBuckets: mockRecentDailyBuckets,
    detailedUsage: {
      today: {
        estimatedCostUsd: 22.8,
        tokens: {
          inputTokens: 18_240_000,
          cachedInputTokens: 0,
          outputTokens: 0,
          reasoningOutputTokens: 0,
          totalTokens: 18_240_000,
        },
      },
      sevenDay: {
        estimatedCostUsd: 115.38,
        tokens: {
          inputTokens: 92_300_000,
          cachedInputTokens: 0,
          outputTokens: 0,
          reasoningOutputTokens: 0,
          totalTokens: 92_300_000,
        },
      },
      month: {
        estimatedCostUsd: 307.5,
        tokens: {
          inputTokens: 246_000_000,
          cachedInputTokens: 0,
          outputTokens: 0,
          reasoningOutputTokens: 0,
          totalTokens: 246_000_000,
        },
      },
      lifetime: {
        estimatedCostUsd: 981.25,
        tokens: {
          inputTokens: 785_000_000,
          cachedInputTokens: 0,
          outputTokens: 0,
          reasoningOutputTokens: 0,
          totalTokens: 785_000_000,
        },
      },
      parsedFileCount: 7,
      tokenEventCount: 7,
    },
  },
  local: {
    lifetimeTokens: 438_200_000,
    todayTokens: 18_240_000,
    sevenDayTokens: 92_300_000,
    threadCount: 128,
    lastUpdatedAt: now - 300,
    dailyBuckets: mockDailyBuckets,
    recentThreads: [],
    detailedUsage: {
      today: {
        estimatedCostUsd: 126.34,
        tokens: {
          inputTokens: 12_000_000,
          cachedInputTokens: 7_200_000,
          outputTokens: 4_600_000,
          reasoningOutputTokens: 0,
          totalTokens: 18_240_000,
        },
      },
      sevenDay: {
        estimatedCostUsd: 734.08,
        tokens: {
          inputTokens: 64_000_000,
          cachedInputTokens: 31_000_000,
          outputTokens: 28_300_000,
          reasoningOutputTokens: 0,
          totalTokens: 92_300_000,
        },
      },
      month: {
        estimatedCostUsd: 1940.42,
        tokens: {
          inputTokens: 180_000_000,
          cachedInputTokens: 91_000_000,
          outputTokens: 66_000_000,
          reasoningOutputTokens: 0,
          totalTokens: 246_000_000,
        },
      },
      valuePeriod: {
        estimatedCostUsd: 734.08,
        tokens: {
          inputTokens: 64_000_000,
          cachedInputTokens: 31_000_000,
          outputTokens: 28_300_000,
          reasoningOutputTokens: 0,
          totalTokens: 92_300_000,
        },
      },
      lifetime: {
        estimatedCostUsd: 4200.86,
        tokens: {
          inputTokens: 320_000_000,
          cachedInputTokens: 161_000_000,
          outputTokens: 118_000_000,
          reasoningOutputTokens: 0,
          totalTokens: 438_200_000,
        },
      },
      parsedFileCount: 84,
      tokenEventCount: 919,
    },
  },
  taskBoard: {
    refreshedAt: now,
    totalCount: 13,
    columns: [
      {
        id: "active",
        title: "进行中",
        count: 3,
        items: [
          {
            id: "a1",
            code: "COD-A921",
            title: "重构 codex-PAISHU 桌面工作台",
            detail: "codex-PAISHU · 880万",
            chip: "高耗",
            updatedAt: now - 300,
            tokens: 8_800_000,
            kind: "active",
          },
        ],
      },
      {
        id: "pending",
        title: "待处理",
        count: 4,
        items: [
          {
            id: "p1",
            code: "COD-44AE",
            title: "校验 Windows 打包产物",
            detail: "发布 · 120万",
            chip: "待机",
            updatedAt: now - 8200,
            tokens: 1_200_000,
            kind: "pending",
          },
        ],
      },
      {
        id: "scheduled",
        title: "定时",
        count: 2,
        items: [
          {
            id: "s1",
            code: "AUTO-MEYO",
            title: "每日工作区心跳",
            detail: "定时 · 每天",
            chip: "定时",
            kind: "scheduled",
          },
        ],
      },
      {
        id: "done",
        title: "完成",
        count: 4,
        items: [
          {
            id: "d1",
            code: "COD-9B1F",
            title: "初始化项目记忆",
            detail: "codexU · 46万",
            chip: "完成",
            updatedAt: now - 14000,
            tokens: 460_000,
            kind: "done",
          },
        ],
      },
    ],
  },
  diagnostics: [
    { id: "codex-cli", title: "Codex CLI", detail: "PATH / codex", status: "ok" },
    { id: "sqlite", title: "SQLite state_5", detail: "~/.codex/state_5.sqlite", status: "ok" },
    { id: "session-logs", title: "会话令牌日志", detail: "已解析 84 个文件", status: "ok" },
  ],
  messages: [],
};
