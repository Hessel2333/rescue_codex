use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub default_codex_root: String,
    pub database_path: String,
    pub session_index_path: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardFilters {
    pub preset: Option<String>,
    pub granularity: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardScope {
    pub preset: String,
    pub granularity: String,
    pub date_from: String,
    pub date_to: String,
    pub available_from: Option<String>,
    pub available_to: Option<String>,
    pub total_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub masked_email: Option<String>,
    pub plan_type: Option<String>,
    pub masked_account_user_id: Option<String>,
    pub current_model: Option<String>,
    pub current_reasoning_effort: Option<String>,
    pub current_speed_tier: Option<String>,
    pub last_refresh: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardOverview {
    pub total_sessions: i64,
    pub total_questions: i64,
    pub active_days: i64,
    pub sessions_last_7_days: i64,
    pub sessions_last_30_days: i64,
    pub avg_duration_sec: f64,
    pub avg_turn_count: f64,
    pub total_tool_calls: i64,
    pub avg_first_response_sec: f64,
    pub avg_turn_completion_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChartDatum {
    pub label: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownDatum {
    pub bucket: String,
    pub category: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolMetricDatum {
    pub label: String,
    pub total: i64,
    pub success: i64,
    pub failure: i64,
    pub avg_duration_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageSummary {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_input_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RankedTurnRecord {
    pub session_id: String,
    pub project: String,
    pub thread_title: Option<String>,
    pub prompt_preview: Option<String>,
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_input_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub first_response_sec: Option<i64>,
    pub completion_sec: Option<i64>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub label: String,
    pub session_count: i64,
    pub question_count: i64,
    pub total_tokens: i64,
    pub context_compactions: i64,
    pub avg_first_response_sec: f64,
    pub avg_completion_sec: f64,
    pub max_parallel_windows: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWindowRecord {
    pub session_id: String,
    pub project: String,
    pub thread_title: Option<String>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub duration_sec: i64,
    pub turn_count: i64,
    pub total_tokens: i64,
    pub question_count: i64,
    pub tool_call_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationDatum {
    pub bucket: String,
    pub sample_count: i64,
    pub avg_first_response_sec: f64,
    pub avg_completion_sec: f64,
    pub avg_total_tokens: f64,
    pub avg_token_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScatterDatum {
    pub x: f64,
    pub completion_sec: f64,
    pub total_tokens: f64,
    pub first_response_sec: f64,
    pub token_rate: f64,
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivityPoint {
    pub date: String,
    pub sessions: i64,
    pub questions: i64,
    pub avg_first_response_sec: i64,
    pub avg_turn_completion_sec: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecentImport {
    pub id: String,
    pub source_label: String,
    pub status: String,
    pub mode: String,
    pub files_total: i64,
    pub files_success: i64,
    pub files_failed: i64,
    pub warnings_count: i64,
    pub errors_count: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportIssueRecord {
    pub id: String,
    pub severity: String,
    pub code: String,
    pub message: String,
    pub line_no: Option<i64>,
    pub raw_excerpt: Option<String>,
    pub created_at: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub thread_title: Option<String>,
    pub cwd: Option<String>,
    pub source: Option<String>,
    pub updated_at: Option<String>,
    pub started_at: Option<String>,
    pub duration_sec: i64,
    pub user_message_count: i64,
    pub assistant_message_count: i64,
    pub tool_call_count: i64,
    pub turn_count: i64,
    pub warning_count: i64,
    pub first_user_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionMessageRecord {
    pub id: String,
    pub turn_id: Option<String>,
    pub role: Option<String>,
    pub kind: String,
    pub text: Option<String>,
    pub ts: Option<String>,
    pub tool_name: Option<String>,
    pub phase: Option<String>,
    pub image_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub session: SessionSummary,
    pub messages: Vec<SessionMessageRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub scope: DashboardScope,
    pub overview: DashboardOverview,
    pub activity: Vec<ActivityPoint>,
    pub daily_activity: Vec<ActivityPoint>,
    pub heatmap_activity: Vec<ActivityPoint>,
    pub project_options: Vec<String>,
    pub selected_project: Option<String>,
    pub project_timeline: Vec<BreakdownDatum>,
    pub project_summaries: Vec<ProjectSummary>,
    pub project_parallelism: Vec<ChartDatum>,
    pub duration_buckets: Vec<ChartDatum>,
    pub question_hours: Vec<ChartDatum>,
    pub first_token_buckets: Vec<ChartDatum>,
    pub completion_buckets: Vec<ChartDatum>,
    pub token_usage: TokenUsageSummary,
    pub top_token_turns: Vec<RankedTurnRecord>,
    pub slowest_turns: Vec<RankedTurnRecord>,
    pub project_windows: Vec<ProjectWindowRecord>,
    pub tool_types: Vec<ChartDatum>,
    pub tool_metrics: Vec<ToolMetricDatum>,
    pub model_usage: Vec<ChartDatum>,
    pub model_timeline: Vec<BreakdownDatum>,
    pub reasoning_efforts: Vec<ChartDatum>,
    pub reasoning_timeline: Vec<BreakdownDatum>,
    pub speed_tiers: Vec<ChartDatum>,
    pub speed_timeline: Vec<BreakdownDatum>,
    pub top_prompt_terms: Vec<ChartDatum>,
    pub prompt_length_buckets: Vec<ChartDatum>,
    pub prompt_composition: Vec<ChartDatum>,
    pub transport_signals: Vec<ChartDatum>,
    pub transport_timeline: Vec<BreakdownDatum>,
    pub interruption_timeline: Vec<BreakdownDatum>,
    pub workspace_switches: Vec<ChartDatum>,
    pub workspace_timeline: Vec<BreakdownDatum>,
    pub hourly_correlations: Vec<CorrelationDatum>,
    pub weekday_correlations: Vec<CorrelationDatum>,
    pub prompt_length_correlations: Vec<CorrelationDatum>,
    pub tool_load_correlations: Vec<CorrelationDatum>,
    pub context_load_correlations: Vec<CorrelationDatum>,
    pub hourly_correlation_scatter: Vec<ScatterDatum>,
    pub weekday_correlation_scatter: Vec<ScatterDatum>,
    pub prompt_length_correlation_scatter: Vec<ScatterDatum>,
    pub tool_load_correlation_scatter: Vec<ScatterDatum>,
    pub context_load_correlation_scatter: Vec<ScatterDatum>,
    pub search_keywords: Vec<ChartDatum>,
    pub search_hours: Vec<ChartDatum>,
    pub top_cwds: Vec<ChartDatum>,
    pub top_sources: Vec<ChartDatum>,
    pub recent_imports: Vec<RecentImport>,
    pub recent_sessions: Vec<SessionSummary>,
    pub recent_issues: Vec<ImportIssueRecord>,
    pub app_info: AppInfo,
    pub account_info: AccountInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionListFilters {
    pub query: Option<String>,
    pub cwd: Option<String>,
    pub source: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponse {
    pub total: i64,
    pub items: Vec<SessionSummary>,
    pub selected: Option<SessionDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportRunResult {
    pub import_id: String,
    pub source_label: String,
    pub root_path: String,
    pub status: String,
    pub files_total: i64,
    pub files_success: i64,
    pub files_failed: i64,
    pub warnings_count: i64,
    pub errors_count: i64,
    pub issues: Vec<ImportIssueRecord>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
    Markdown,
}

impl Default for ExportFormat {
    fn default() -> Self {
        Self::Json
    }
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Markdown => "md",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportKind {
    Dashboard,
    Sessions,
}

impl Default for ExportKind {
    fn default() -> Self {
        Self::Dashboard
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub kind: ExportKind,
    pub format: ExportFormat,
    pub path: String,
    pub filters: Option<SessionListFilters>,
    pub dashboard_filters: Option<DashboardFilters>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub id: String,
    pub path: String,
    pub format: ExportFormat,
    pub bytes_written: u64,
}
