export type ThemeMode = "system" | "light" | "dark";
export type LanguageMode = "auto" | "zh" | "en";
export type DiagnosticStatus = "ok" | "warning" | "error" | "unknown";
export type TaskColumnKind = "active" | "pending" | "scheduled" | "done";
export type CodexAccessMode = "official" | "relay";
export type ReasoningEffort = "minimal" | "low" | "medium" | "high" | "extreme";
export type ApiSpeedMode = "stable" | "balanced" | "fast";
export type ValuePeriodSource = "membershipStart" | "officialRecord" | "calendarMonth";

export interface RateWindow {
  usedPercent: number;
  windowDurationMins?: number | null;
  resetsAt?: number | null;
}

export interface CreditsInfo {
  hasCredits: boolean;
  unlimited: boolean;
  balance?: string | null;
  resetCredits?: number | null;
}

export interface AccountInfo {
  accountType: string;
  planType?: string | null;
  emailPresent: boolean;
}

export interface TokenBreakdown {
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
  reasoningOutputTokens: number;
  totalTokens: number;
}

export interface PricedTokenUsage {
  tokens: TokenBreakdown;
  estimatedCostUsd: number;
}

export interface DetailedUsage {
  today: PricedTokenUsage;
  sevenDay: PricedTokenUsage;
  month: PricedTokenUsage;
  valuePeriod?: PricedTokenUsage | null;
  lifetime: PricedTokenUsage;
  parsedFileCount: number;
  tokenEventCount: number;
}

export interface LocalThread {
  id: string;
  title: string;
  tokens: number;
  updatedAt?: number | null;
  model?: string | null;
  cwd: string;
  archived: boolean;
}

export interface DailyTokenBucket {
  id: string;
  label: string;
  tokens: number;
}

export interface LocalUsage {
  lifetimeTokens: number;
  todayTokens: number;
  sevenDayTokens: number;
  threadCount: number;
  lastUpdatedAt?: number | null;
  dailyBuckets: DailyTokenBucket[];
  recentThreads: LocalThread[];
  detailedUsage?: DetailedUsage | null;
}

export interface OfficialUsage {
  lifetimeTokens: number;
  todayTokens: number;
  sevenDayTokens: number;
  monthTokens: number;
  valuePeriodStart?: string | null;
  valuePeriodSource: ValuePeriodSource;
  valuePeriodTokens: number;
  valuePeriod: PricedTokenUsage;
  dailyBuckets: DailyTokenBucket[];
  recentDailyBuckets: DailyTokenBucket[];
  detailedUsage: DetailedUsage;
}

export interface TaskItem {
  id: string;
  code: string;
  title: string;
  detail: string;
  chip: string;
  updatedAt?: number | null;
  tokens?: number | null;
  kind: TaskColumnKind;
}

export interface TaskColumn {
  id: TaskColumnKind;
  title: string;
  count: number;
  items: TaskItem[];
}

export interface TaskBoard {
  refreshedAt: number;
  columns: TaskColumn[];
  totalCount: number;
}

export interface DiagnosticItem {
  id: string;
  title: string;
  detail: string;
  status: DiagnosticStatus;
}

export interface UsageSnapshot {
  refreshedAt: number;
  account?: AccountInfo | null;
  limitId?: string | null;
  limitName?: string | null;
  primary?: RateWindow | null;
  secondary?: RateWindow | null;
  credits?: CreditsInfo | null;
  cloudLifetimeTokens?: number | null;
  officialUsage?: OfficialUsage | null;
  local?: LocalUsage | null;
  taskBoard?: TaskBoard | null;
  diagnostics: DiagnosticItem[];
  messages: string[];
}

export interface AppSettings {
  language: LanguageMode;
  theme: ThemeMode;
  alwaysOnTop: boolean;
  showOnStart: boolean;
  codexBinaryPath?: string | null;
  codexDataDir?: string | null;
  refreshIntervalSecs: number;
  showTaskBoard: boolean;
  accessMode: CodexAccessMode;
  apiEndpoint?: string | null;
  apiKey?: string | null;
  apiModel: string;
  reasoningEffort: ReasoningEffort;
  speedMode: ApiSpeedMode;
  membershipStartedOn?: string | null;
}

export interface DetectionPaths {
  codexBinaryPath?: string | null;
  codexDataDir?: string | null;
  stateDbPath?: string | null;
  appLogDir: string;
}

export interface CodexConfigBackup {
  id: string;
  label: string;
  createdAt: string;
  isDefault: boolean;
  hasConfig: boolean;
  hasAuth: boolean;
}
