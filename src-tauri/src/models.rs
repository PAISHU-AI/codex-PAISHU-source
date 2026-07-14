use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateWindow {
    pub used_percent: f64,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreditsInfo {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
    pub reset_credits: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub account_type: String,
    pub plan_type: Option<String>,
    pub email_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub is_logged_in: bool,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalThread {
    pub id: String,
    pub title: String,
    pub tokens: i64,
    pub updated_at: Option<i64>,
    pub model: Option<String>,
    pub cwd: String,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyTokenBucket {
    pub id: String,
    pub label: String,
    pub tokens: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenBreakdown {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
}

impl TokenBreakdown {
    pub fn add(&mut self, other: Self) {
        self.input_tokens += other.input_tokens;
        self.cached_input_tokens += other.cached_input_tokens;
        self.output_tokens += other.output_tokens;
        self.reasoning_output_tokens += other.reasoning_output_tokens;
        self.total_tokens += other.total_tokens;
    }

    pub fn delta(self, previous: Self) -> Self {
        Self {
            input_tokens: self.input_tokens - previous.input_tokens,
            cached_input_tokens: self.cached_input_tokens - previous.cached_input_tokens,
            output_tokens: self.output_tokens - previous.output_tokens,
            reasoning_output_tokens: self.reasoning_output_tokens
                - previous.reasoning_output_tokens,
            total_tokens: self.total_tokens - previous.total_tokens,
        }
    }

    pub fn has_negative_value(&self) -> bool {
        self.input_tokens < 0
            || self.cached_input_tokens < 0
            || self.output_tokens < 0
            || self.reasoning_output_tokens < 0
            || self.total_tokens < 0
    }

    pub fn is_zero(&self) -> bool {
        self.input_tokens == 0
            && self.cached_input_tokens == 0
            && self.output_tokens == 0
            && self.reasoning_output_tokens == 0
            && self.total_tokens == 0
    }

    pub fn billable_cached_input_tokens(&self) -> i64 {
        self.cached_input_tokens
            .max(0)
            .min(self.input_tokens.max(0))
    }

    pub fn uncached_input_tokens(&self) -> i64 {
        (self.input_tokens - self.billable_cached_input_tokens()).max(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PricedTokenUsage {
    pub tokens: TokenBreakdown,
    pub estimated_cost_usd: f64,
}

impl PricedTokenUsage {
    pub fn add(&mut self, tokens: TokenBreakdown, cost_usd: f64) {
        self.tokens.add(tokens);
        self.estimated_cost_usd += cost_usd;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DetailedUsage {
    pub five_hour_local: Option<PricedTokenUsage>,
    pub today: PricedTokenUsage,
    pub seven_day: PricedTokenUsage,
    pub month: PricedTokenUsage,
    pub value_period: Option<PricedTokenUsage>,
    pub lifetime: PricedTokenUsage,
    pub parsed_file_count: usize,
    pub token_event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsage {
    pub lifetime_tokens: i64,
    pub today_tokens: i64,
    pub seven_day_tokens: i64,
    pub thread_count: i64,
    pub last_updated_at: Option<i64>,
    pub daily_buckets: Vec<DailyTokenBucket>,
    pub recent_threads: Vec<LocalThread>,
    pub detailed_usage: Option<DetailedUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OfficialUsage {
    pub lifetime_tokens: i64,
    pub today_tokens: i64,
    pub seven_day_tokens: i64,
    pub month_tokens: i64,
    pub value_period_start: Option<String>,
    pub value_period_source: ValuePeriodSource,
    pub value_period_tokens: i64,
    pub value_period: PricedTokenUsage,
    pub daily_buckets: Vec<DailyTokenBucket>,
    pub recent_daily_buckets: Vec<DailyTokenBucket>,
    pub detailed_usage: DetailedUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ValuePeriodSource {
    MembershipStart,
    OfficialRecord,
    #[default]
    CalendarMonth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskColumnKind {
    Active,
    Pending,
    Scheduled,
    Done,
}

impl Default for TaskColumnKind {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskItem {
    pub id: String,
    pub code: String,
    pub title: String,
    pub detail: String,
    pub chip: String,
    pub updated_at: Option<i64>,
    pub tokens: Option<i64>,
    pub kind: TaskColumnKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskColumn {
    pub id: TaskColumnKind,
    pub title: String,
    pub count: usize,
    pub items: Vec<TaskItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskBoard {
    pub refreshed_at: i64,
    pub columns: Vec<TaskColumn>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticItem {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub status: DiagnosticStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticStatus {
    Ok,
    Warning,
    Error,
    Unknown,
}

impl Default for DiagnosticStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub refreshed_at: i64,
    pub account: Option<AccountInfo>,
    pub auth_status: AuthStatus,
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub primary: Option<RateWindow>,
    pub secondary: Option<RateWindow>,
    pub credits: Option<CreditsInfo>,
    pub cloud_lifetime_tokens: Option<i64>,
    pub official_usage: Option<OfficialUsage>,
    pub local: Option<LocalUsage>,
    pub task_board: Option<TaskBoard>,
    pub diagnostics: Vec<DiagnosticItem>,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LanguageMode {
    Auto,
    Zh,
    En,
}

impl Default for LanguageMode {
    fn default() -> Self {
        Self::Zh
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodexAccessMode {
    Official,
    Relay,
}

impl Default for CodexAccessMode {
    fn default() -> Self {
        Self::Official
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    Extreme,
}

impl Default for ReasoningEffort {
    fn default() -> Self {
        Self::Medium
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ApiSpeedMode {
    Stable,
    Balanced,
    Fast,
}

impl Default for ApiSpeedMode {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppSettings {
    pub language: LanguageMode,
    pub theme: ThemeMode,
    pub always_on_top: bool,
    pub show_on_start: bool,
    pub codex_binary_path: Option<String>,
    pub codex_data_dir: Option<String>,
    pub refresh_interval_secs: u64,
    pub show_task_board: bool,
    pub access_mode: CodexAccessMode,
    pub api_endpoint: Option<String>,
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
    pub api_model: String,
    pub reasoning_effort: ReasoningEffort,
    pub speed_mode: ApiSpeedMode,
    pub membership_started_on: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: LanguageMode::Auto,
            theme: ThemeMode::System,
            always_on_top: false,
            show_on_start: true,
            codex_binary_path: None,
            codex_data_dir: None,
            refresh_interval_secs: 300,
            show_task_board: true,
            access_mode: CodexAccessMode::Official,
            api_endpoint: None,
            api_key: None,
            api_model: "gpt-5".to_string(),
            reasoning_effort: ReasoningEffort::Medium,
            speed_mode: ApiSpeedMode::Balanced,
            membership_started_on: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DetectionPaths {
    pub codex_binary_path: Option<String>,
    pub codex_data_dir: Option<String>,
    pub state_db_path: Option<String>,
    pub app_log_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexConfigBackup {
    pub id: String,
    pub label: String,
    pub created_at: String,
    pub is_default: bool,
    pub has_config: bool,
    pub has_auth: bool,
}
