use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Utc};
use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection};
use serde_json::Value as JsonValue;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
};

use crate::{
    db::open_connection,
    models::api::{
        AccountInfo, ActivityPoint, AppInfo, BreakdownDatum, ChartDatum, CorrelationDatum,
        DashboardFilters, DashboardOverview, DashboardScope, DashboardSummary, ImportIssueRecord,
        ProjectSummary, ProjectWindowRecord, RankedTurnRecord, RecentImport, ScatterDatum,
        SessionDetail, SessionListFilters, SessionListResponse, SessionMessageRecord,
        SessionSummary, TokenUsageSummary, ToolMetricDatum,
    },
    state::AppState,
};

const SESSION_DATE_SQL: &str = "substr(COALESCE(updated_at, started_at, ''), 1, 10)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeGranularity {
    Day,
    Week,
    Month,
    Year,
}

impl TimeGranularity {
    fn from_filter(value: Option<&str>) -> Self {
        match value.unwrap_or("day").trim().to_ascii_lowercase().as_str() {
            "week" => Self::Week,
            "month" => Self::Month,
            "year" => Self::Year,
            _ => Self::Day,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RangePreset {
    Days7,
    Days30,
    Days90,
    Days365,
    All,
    Custom,
}

impl RangePreset {
    fn from_filter(value: Option<&str>) -> Self {
        match value.unwrap_or("30d").trim().to_ascii_lowercase().as_str() {
            "7d" => Self::Days7,
            "90d" => Self::Days90,
            "365d" => Self::Days365,
            "all" => Self::All,
            "custom" => Self::Custom,
            _ => Self::Days30,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Days7 => "7d",
            Self::Days30 => "30d",
            Self::Days90 => "90d",
            Self::Days365 => "365d",
            Self::All => "all",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SummarySection {
    All,
    Overview,
    Performance,
    Workflow,
    Search,
    Projects,
    Correlations,
}

impl SummarySection {
    fn from_filter(value: Option<&str>) -> Self {
        match value.unwrap_or("all").trim().to_ascii_lowercase().as_str() {
            "overview" => Self::Overview,
            "performance" => Self::Performance,
            "workflow" => Self::Workflow,
            "search" => Self::Search,
            "projects" => Self::Projects,
            "correlations" => Self::Correlations,
            _ => Self::All,
        }
    }

    fn includes_overview(self) -> bool {
        matches!(self, Self::All | Self::Overview)
    }

    fn includes_performance(self) -> bool {
        matches!(self, Self::All | Self::Performance)
    }

    fn includes_workflow(self) -> bool {
        matches!(self, Self::All | Self::Workflow)
    }

    fn includes_search(self) -> bool {
        matches!(self, Self::All | Self::Search)
    }

    fn includes_projects(self) -> bool {
        matches!(self, Self::All | Self::Projects)
    }

    fn includes_correlations(self) -> bool {
        matches!(self, Self::All | Self::Correlations)
    }
}

#[derive(Debug, Clone)]
struct ResolvedDashboardScope {
    preset: RangePreset,
    granularity: TimeGranularity,
    start: NaiveDate,
    end: NaiveDate,
    available_start: Option<NaiveDate>,
    available_end: Option<NaiveDate>,
}

impl ResolvedDashboardScope {
    fn contains_date(&self, date: NaiveDate) -> bool {
        date >= self.start && date <= self.end
    }

    fn to_api(&self) -> DashboardScope {
        DashboardScope {
            preset: self.preset.as_str().to_string(),
            granularity: self.granularity.as_str().to_string(),
            date_from: self.start.format("%Y-%m-%d").to_string(),
            date_to: self.end.format("%Y-%m-%d").to_string(),
            available_from: self
                .available_start
                .map(|value| value.format("%Y-%m-%d").to_string()),
            available_to: self
                .available_end
                .map(|value| value.format("%Y-%m-%d").to_string()),
            total_days: (self.end - self.start).num_days() + 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct TurnRecord {
    session_id: String,
    started_at: Option<DateTime<Utc>>,
    user_ts: Option<DateTime<Utc>>,
    first_assistant_ts: Option<DateTime<Utc>>,
    completed_ts: Option<DateTime<Utc>>,
    task_duration_ms: Option<i64>,
    model: Option<String>,
    effort: Option<String>,
    cwd: Option<String>,
    user_text: Option<String>,
    turn_context_count: i64,
    aborted: bool,
    tool_call_count: i64,
    input_tokens: i64,
    output_tokens: i64,
    cached_input_tokens: i64,
    reasoning_output_tokens: i64,
    total_tokens: i64,
    last_token_signature: Option<String>,
}

impl TurnRecord {
    fn first_response_sec(&self) -> Option<i64> {
        self.user_ts
            .zip(self.first_assistant_ts)
            .map(|(start, end)| (end - start).num_seconds().max(0))
    }

    fn completion_sec(&self) -> Option<i64> {
        if let Some(duration_ms) = self.task_duration_ms {
            return Some((duration_ms / 1000).max(0));
        }

        self.user_ts
            .or(self.started_at)
            .zip(self.completed_ts)
            .map(|(start, end)| (end - start).num_seconds().max(0))
    }

    fn anchor_ts(&self) -> Option<DateTime<Utc>> {
        self.user_ts
            .or(self.started_at)
            .or(self.completed_ts)
            .or(self.first_assistant_ts)
    }

    fn local_date(&self) -> Option<NaiveDate> {
        self.anchor_ts().map(to_local_date)
    }

    fn prompt_length(&self) -> usize {
        self.user_text
            .as_deref()
            .map(str::chars)
            .map(Iterator::count)
            .unwrap_or(0)
    }

    fn token_rate(&self) -> Option<f64> {
        let completion = self.completion_sec()?;
        if completion <= 0 || self.total_tokens <= 0 {
            None
        } else {
            Some(self.total_tokens as f64 / completion as f64)
        }
    }
}

#[derive(Debug, Clone, Default)]
struct TurnAnalytics {
    turns: Vec<TurnRecord>,
    tool_types: HashMap<String, i64>,
    tool_metrics: HashMap<String, ToolMetricAccumulator>,
    models: HashMap<String, i64>,
    efforts: HashMap<String, i64>,
    prompt_terms: HashMap<String, i64>,
    search_terms: HashMap<String, i64>,
    search_hours: [i64; 24],
    prompt_lengths: Vec<usize>,
    prompts_with_code: i64,
    prompts_with_path: i64,
    prompts_with_command: i64,
    prompts_with_path_or_command: i64,
    repeated_turn_contexts: i64,
    context_compactions: i64,
    aborted_turns: i64,
    rolled_back_turns: i64,
    project_switches: i64,
    workspace_switches: i64,
    model_timeline: BTreeMap<String, HashMap<String, i64>>,
    effort_timeline: BTreeMap<String, HashMap<String, i64>>,
    interruption_timeline: BTreeMap<String, HashMap<String, i64>>,
    workspace_timeline: BTreeMap<String, HashMap<String, i64>>,
    project_compactions: HashMap<String, i64>,
}

#[derive(Debug, Clone, Default)]
struct ToolMetricAccumulator {
    total: i64,
    success: i64,
    failure: i64,
    duration_total_sec: f64,
    duration_samples: i64,
}

#[derive(Debug, Clone, Default)]
struct TokenUsageAccumulator {
    input_tokens: i64,
    output_tokens: i64,
    cached_input_tokens: i64,
    reasoning_output_tokens: i64,
    total_tokens: i64,
}

#[derive(Debug, Clone, Default)]
struct CorrelationAccumulator {
    sample_count: i64,
    first_response_total: f64,
    first_response_samples: i64,
    completion_total: f64,
    completion_samples: i64,
    total_tokens: i64,
    token_rate_total: f64,
    token_rate_samples: i64,
}

#[derive(Debug, Clone, Copy)]
enum ScatterDimension {
    HourOfDay,
    WeekendFlag,
    PromptLength,
    ToolLoad,
    ContextLoad,
}

#[derive(Debug, Clone, Default)]
struct LogAnalytics {
    signals: HashMap<String, i64>,
    transport_timeline: BTreeMap<String, HashMap<String, i64>>,
    speed_tiers: HashMap<String, i64>,
    speed_timeline: BTreeMap<String, HashMap<String, i64>>,
}

#[derive(Debug, Clone)]
struct SessionSnapshot {
    id: String,
    date: NaiveDate,
    thread_title: Option<String>,
    cwd: Option<String>,
    source: Option<String>,
    updated_at: Option<String>,
    started_at: Option<String>,
    duration_sec: i64,
    user_message_count: i64,
    assistant_message_count: i64,
    tool_call_count: i64,
    turn_count: i64,
    warning_count: i64,
    first_user_message: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ActivityAccumulator {
    sessions: i64,
    questions: i64,
    first_response_total: i64,
    first_response_samples: i64,
    completion_total: i64,
    completion_samples: i64,
}

pub fn get_dashboard_summary(
    state: &AppState,
    filters: DashboardFilters,
    section: Option<String>,
) -> anyhow::Result<DashboardSummary> {
    let conn = open_connection(state.db_path())?;
    load_dashboard_summary(&conn, state, filters, section)
}

pub fn list_sessions(
    state: &AppState,
    filters: SessionListFilters,
) -> anyhow::Result<SessionListResponse> {
    let conn = open_connection(state.db_path())?;
    load_sessions(&conn, &filters)
}

pub(crate) fn load_dashboard_summary(
    conn: &Connection,
    state: &AppState,
    filters: DashboardFilters,
    section: Option<String>,
) -> anyhow::Result<DashboardSummary> {
    let section = SummarySection::from_filter(section.as_deref());
    let scope = resolve_dashboard_scope(conn, &filters)?;
    let selected_project = normalize_project_filter(filters.project.as_deref());
    let session_snapshots = load_session_snapshots(conn, &scope, selected_project.as_deref())?;
    let mut summary = DashboardSummary {
        scope: scope.to_api(),
        app_info: build_app_info(state),
        account_info: build_account_info(),
        project_options: load_project_options(conn)?,
        selected_project: filters
            .project
            .clone()
            .filter(|value| !value.trim().is_empty() && !value.eq_ignore_ascii_case("all")),
        ..Default::default()
    };

    let needs_turn_analytics = section.includes_overview()
        || section.includes_performance()
        || section.includes_workflow()
        || section.includes_search()
        || section.includes_projects()
        || section.includes_correlations();
    let turn_analytics = if needs_turn_analytics {
        Some(build_turn_analytics(
            conn,
            &scope,
            selected_project.as_deref(),
            &session_snapshots,
        )?)
    } else {
        None
    };

    let needs_log_analytics = section.includes_performance() || section.includes_workflow();
    let log_analytics = if needs_log_analytics {
        Some(build_log_analytics(&scope, selected_project.as_deref())?)
    } else {
        None
    };

    if section.includes_overview() {
        let overview_turns = turn_analytics.as_ref().expect("overview turn analytics");
        let daily_activity_map =
            build_daily_activity_map(&scope, &session_snapshots, overview_turns);
        let heatmap_scope = build_heatmap_scope(&scope);
        let heatmap_snapshots =
            load_session_snapshots(conn, &heatmap_scope, selected_project.as_deref())?;
        let heatmap_turn_analytics = build_turn_analytics(
            conn,
            &heatmap_scope,
            selected_project.as_deref(),
            &heatmap_snapshots,
        )?;

        summary.activity = aggregate_activity(&daily_activity_map, scope.granularity);
        summary.daily_activity = activity_points_from_daily_map(&daily_activity_map);
        summary.heatmap_activity = activity_points_from_daily_map(&build_daily_activity_map(
            &heatmap_scope,
            &heatmap_snapshots,
            &heatmap_turn_analytics,
        ));
        summary.overview = build_overview(&session_snapshots, overview_turns)?;
        summary.top_cwds =
            top_metrics_from_sessions(&session_snapshots, |item| item.cwd.clone(), 6);
        summary.top_sources =
            top_metrics_from_sessions(&session_snapshots, |item| item.source.clone(), 6);
        summary.recent_imports = load_recent_imports(conn)?;
        summary.recent_sessions = recent_sessions_from_snapshots(&session_snapshots, 8);
        summary.recent_issues = load_recent_issues(conn)?;
    }

    if section.includes_performance() {
        let performance_turns = turn_analytics.as_ref().expect("performance turn analytics");
        let performance_logs = log_analytics.as_ref().expect("performance log analytics");
        summary.duration_buckets = build_duration_buckets(&session_snapshots)?;
        summary.first_token_buckets = build_first_response_buckets(performance_turns);
        summary.completion_buckets = build_completion_buckets(performance_turns);
        summary.tool_types = top_chart_data(&performance_turns.tool_types, 8);
        summary.tool_metrics = build_tool_metrics(&performance_turns.tool_metrics, 8);
        summary.model_usage = top_chart_data(&performance_turns.models, 8);
        summary.model_timeline = build_breakdown_series(
            &performance_turns.model_timeline,
            &top_labels(&performance_turns.models, 5),
        );
        summary.reasoning_efforts = top_chart_data(&performance_turns.efforts, 6);
        summary.reasoning_timeline = build_breakdown_series(
            &performance_turns.effort_timeline,
            &top_labels(&performance_turns.efforts, 4),
        );
        summary.speed_tiers = top_chart_data(&performance_logs.speed_tiers, 4);
        summary.speed_timeline = build_breakdown_series(
            &performance_logs.speed_timeline,
            &top_labels(&performance_logs.speed_tiers, 4),
        );
        summary.token_usage = build_token_usage_summary(performance_turns);
        summary.top_token_turns = build_ranked_turns(
            performance_turns,
            &session_snapshots,
            RankedTurnMode::ByTokens,
            10,
        );
        summary.slowest_turns = build_ranked_turns(
            performance_turns,
            &session_snapshots,
            RankedTurnMode::ByCompletionLatency,
            10,
        );
    }

    if section.includes_workflow() {
        let workflow_turns = turn_analytics.as_ref().expect("workflow turn analytics");
        let workflow_logs = log_analytics.as_ref().expect("workflow log analytics");
        summary.question_hours = build_question_hours(workflow_turns);
        summary.top_prompt_terms =
            top_chart_data_with_min_count(&workflow_turns.prompt_terms, 10, 2);
        summary.prompt_length_buckets = build_prompt_length_buckets(workflow_turns);
        summary.prompt_composition = build_prompt_composition(workflow_turns);
        summary.transport_signals = build_transport_signals(workflow_logs);
        summary.transport_timeline = build_breakdown_series(
            &workflow_logs.transport_timeline,
            &[
                "Reconnect".to_string(),
                "Retry".to_string(),
                "Transport error".to_string(),
            ],
        );
        summary.interruption_timeline = build_breakdown_series(
            &workflow_turns.interruption_timeline,
            &[
                "Turn aborted".to_string(),
                "Rollback".to_string(),
                "Context compacted".to_string(),
            ],
        );
        summary.workspace_switches = build_workspace_switches(workflow_turns);
        summary.workspace_timeline = build_breakdown_series(
            &workflow_turns.workspace_timeline,
            &["Project switch".to_string(), "Workspace switch".to_string()],
        );
    }

    if section.includes_correlations() {
        let correlation_turns = turn_analytics.as_ref().expect("correlation turn analytics");
        summary.hourly_correlations = build_hourly_correlations(correlation_turns);
        summary.weekday_correlations = build_weekday_correlations(correlation_turns);
        summary.prompt_length_correlations = build_prompt_length_correlations(correlation_turns);
        summary.tool_load_correlations = build_tool_load_correlations(correlation_turns);
        summary.context_load_correlations = build_context_load_correlations(correlation_turns);
        summary.hourly_correlation_scatter =
            build_correlation_scatter(correlation_turns, ScatterDimension::HourOfDay);
        summary.weekday_correlation_scatter =
            build_correlation_scatter(correlation_turns, ScatterDimension::WeekendFlag);
        summary.prompt_length_correlation_scatter =
            build_correlation_scatter(correlation_turns, ScatterDimension::PromptLength);
        summary.tool_load_correlation_scatter =
            build_correlation_scatter(correlation_turns, ScatterDimension::ToolLoad);
        summary.context_load_correlation_scatter =
            build_correlation_scatter(correlation_turns, ScatterDimension::ContextLoad);
    }

    if section.includes_search() {
        let search_turns = turn_analytics.as_ref().expect("search turn analytics");
        summary.search_keywords = top_chart_data_with_min_count(&search_turns.search_terms, 10, 1);
        summary.search_hours = build_search_hours(search_turns);
    }

    if section.includes_projects() {
        let project_turns = turn_analytics.as_ref().expect("project turn analytics");
        summary.project_timeline =
            build_project_timeline(&scope, &session_snapshots, project_turns);
        summary.project_summaries = build_project_summaries(&session_snapshots, project_turns, 12);
        summary.project_parallelism = build_project_parallelism(&session_snapshots, 12);
        summary.project_windows = build_project_windows(&session_snapshots, project_turns, 12);
    }

    Ok(summary)
}

pub(crate) fn load_sessions(
    conn: &Connection,
    filters: &SessionListFilters,
) -> anyhow::Result<SessionListResponse> {
    let mut where_clauses = vec!["1 = 1".to_string()];
    let mut values = Vec::<SqlValue>::new();

    if let Some(query) = filters
        .query
        .as_ref()
        .filter(|query| !query.trim().is_empty())
    {
        where_clauses
            .push("(thread_title LIKE ? OR cwd LIKE ? OR first_user_message LIKE ?)".to_string());
        let like = format!("%{}%", query.trim());
        values.push(SqlValue::from(like.clone()));
        values.push(SqlValue::from(like.clone()));
        values.push(SqlValue::from(like));
    }

    if let Some(cwd) = filters.cwd.as_ref().filter(|cwd| !cwd.trim().is_empty()) {
        where_clauses.push("cwd LIKE ?".to_string());
        values.push(SqlValue::from(format!("%{}%", cwd.trim())));
    }

    if let Some(source) = filters
        .source
        .as_ref()
        .filter(|source| !source.trim().is_empty())
    {
        where_clauses.push("source LIKE ?".to_string());
        values.push(SqlValue::from(format!("%{}%", source.trim())));
    }

    if let Some(date_from) = filters
        .date_from
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        where_clauses.push(format!("{SESSION_DATE_SQL} >= ?"));
        values.push(SqlValue::from(date_from.clone()));
    }

    if let Some(date_to) = filters
        .date_to
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        where_clauses.push(format!("{SESSION_DATE_SQL} <= ?"));
        values.push(SqlValue::from(date_to.clone()));
    }

    let where_sql = where_clauses.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM sessions WHERE {where_sql}");
    let total = conn.query_row(&count_sql, params_from_iter(values.iter()), |row| {
        row.get(0)
    })?;

    let mut item_values = values.clone();
    let limit = filters.limit.unwrap_or(25) as i64;
    let offset = filters.offset.unwrap_or(0) as i64;
    item_values.push(SqlValue::from(limit));
    item_values.push(SqlValue::from(offset));

    let list_sql = format!(
        "SELECT id, thread_title, cwd, source, updated_at, started_at, duration_sec, user_message_count,
                assistant_message_count, tool_call_count, turn_count, warning_count, first_user_message
         FROM sessions
         WHERE {where_sql}
         ORDER BY COALESCE(updated_at, started_at) DESC
         LIMIT ? OFFSET ?"
    );

    let mut stmt = conn.prepare(&list_sql)?;
    let rows = stmt.query_map(params_from_iter(item_values.iter()), map_session_summary)?;
    let items = rows.filter_map(Result::ok).collect::<Vec<_>>();

    let selected = if let Some(session_id) = filters.session_id.as_ref() {
        let session = fetch_session_summary(conn, session_id)?;
        let messages = fetch_session_messages(conn, session_id)?;
        Some(SessionDetail { session, messages })
    } else {
        None
    };

    Ok(SessionListResponse {
        total,
        items,
        selected,
    })
}

fn resolve_dashboard_scope(
    conn: &Connection,
    filters: &DashboardFilters,
) -> anyhow::Result<ResolvedDashboardScope> {
    let selected_project = normalize_project_filter(filters.project.as_deref());
    let mut stmt = conn.prepare(&format!(
        "SELECT COALESCE(updated_at, started_at) AS ts, cwd
         FROM sessions
         WHERE {SESSION_DATE_SQL} <> ''"
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
        ))
    })?;
    let mut available_start: Option<NaiveDate> = None;
    let mut available_end: Option<NaiveDate> = None;

    for row in rows.filter_map(Result::ok) {
        let (ts, cwd) = row;
        if !matches_project_filter(cwd.as_deref(), selected_project.as_deref()) {
            continue;
        }
        let Some(date) = ts
            .as_deref()
            .and_then(parse_rfc3339_utc)
            .map(to_local_date)
            .or_else(|| {
                ts.as_deref()
                    .and_then(|value| value.get(0..10))
                    .and_then(parse_naive_date)
            })
        else {
            continue;
        };
        available_start = Some(available_start.map(|value| value.min(date)).unwrap_or(date));
        available_end = Some(available_end.map(|value| value.max(date)).unwrap_or(date));
    }

    let today = Local::now().date_naive();
    let preset = RangePreset::from_filter(filters.preset.as_deref());
    let granularity = TimeGranularity::from_filter(filters.granularity.as_deref());

    let (mut start, mut end) = match preset {
        RangePreset::Days7 => (today - Duration::days(6), today),
        RangePreset::Days30 => (today - Duration::days(29), today),
        RangePreset::Days90 => (today - Duration::days(89), today),
        RangePreset::Days365 => (today - Duration::days(364), today),
        RangePreset::All => (
            available_start.unwrap_or(today),
            available_end.unwrap_or(today),
        ),
        RangePreset::Custom => (
            filters
                .date_from
                .as_deref()
                .and_then(parse_naive_date)
                .or(available_start)
                .unwrap_or(today),
            filters
                .date_to
                .as_deref()
                .and_then(parse_naive_date)
                .or(available_end)
                .unwrap_or(today),
        ),
    };

    if start > end {
        std::mem::swap(&mut start, &mut end);
    }

    Ok(ResolvedDashboardScope {
        preset,
        granularity,
        start,
        end,
        available_start,
        available_end,
    })
}

fn build_heatmap_scope(scope: &ResolvedDashboardScope) -> ResolvedDashboardScope {
    let today = Local::now().date_naive();
    let available_end = scope.available_end.unwrap_or(scope.end).min(today);
    let candidate_start = available_end - Duration::days(370);
    let start = scope
        .available_start
        .map(|value| value.max(candidate_start))
        .unwrap_or(candidate_start);

    ResolvedDashboardScope {
        preset: RangePreset::Custom,
        granularity: TimeGranularity::Day,
        start,
        end: available_end,
        available_start: scope.available_start,
        available_end: Some(available_end),
    }
}

fn normalize_project_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("all"))
        .map(|value| value.to_ascii_lowercase())
}

fn project_label_from_cwd(value: &str) -> String {
    last_path_component(value)
        .filter(|item| !item.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.trim().to_string())
}

fn project_label_from_option(value: Option<&str>) -> String {
    value
        .map(project_label_from_cwd)
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn matches_project_filter(cwd: Option<&str>, selected_project: Option<&str>) -> bool {
    match selected_project {
        None => true,
        Some(selected_project) => cwd
            .map(normalize_workspace)
            .map(|value| project_key(&value))
            .is_some_and(|value| value == selected_project),
    }
}

fn load_project_options(conn: &Connection) -> anyhow::Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT DISTINCT cwd FROM sessions WHERE COALESCE(cwd, '') <> ''")?;
    let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;
    let mut labels = HashMap::<String, String>::new();
    for cwd in rows.filter_map(Result::ok).flatten() {
        let normalized = normalize_workspace(&cwd);
        let key = project_key(&normalized);
        labels
            .entry(key)
            .or_insert_with(|| project_label_from_cwd(&cwd));
    }
    let mut items = labels.into_values().collect::<Vec<_>>();
    items.sort();
    Ok(items)
}

fn build_overview(
    session_snapshots: &[SessionSnapshot],
    turn_analytics: &TurnAnalytics,
) -> anyhow::Result<DashboardOverview> {
    let total_sessions = session_snapshots.len() as i64;
    let avg_duration_sec = average_metric(
        session_snapshots
            .iter()
            .map(|item| item.duration_sec as f64),
    );
    let avg_turn_count =
        average_metric(session_snapshots.iter().map(|item| item.turn_count as f64));
    let total_tool_calls = session_snapshots
        .iter()
        .map(|item| item.tool_call_count)
        .sum();

    let active_days = session_snapshots
        .iter()
        .map(|snapshot| snapshot.date)
        .collect::<HashSet<_>>()
        .len() as i64;
    let range_end = session_snapshots
        .iter()
        .map(|snapshot| snapshot.date)
        .max()
        .unwrap_or_else(|| Local::now().date_naive());
    let range_7 = range_end - Duration::days(6);
    let range_30 = range_end - Duration::days(29);
    let sessions_last_7_days = session_snapshots
        .iter()
        .filter(|snapshot| snapshot.date >= range_7)
        .count() as i64;
    let sessions_last_30_days = session_snapshots
        .iter()
        .filter(|snapshot| snapshot.date >= range_30)
        .count() as i64;

    let question_turns = turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
        .collect::<Vec<_>>();
    let total_questions = question_turns.len() as i64;
    let avg_first_response_sec = average_metric(
        question_turns
            .iter()
            .filter_map(|turn| turn.first_response_sec().map(|value| value as f64)),
    );
    let avg_turn_completion_sec = average_metric(
        question_turns
            .iter()
            .filter_map(|turn| turn.completion_sec().map(|value| value as f64)),
    );

    Ok(DashboardOverview {
        total_sessions,
        total_questions,
        active_days,
        sessions_last_7_days,
        sessions_last_30_days,
        avg_duration_sec,
        avg_turn_count,
        total_tool_calls,
        avg_first_response_sec,
        avg_turn_completion_sec,
    })
}

fn load_session_snapshots(
    conn: &Connection,
    scope: &ResolvedDashboardScope,
    selected_project: Option<&str>,
) -> anyhow::Result<Vec<SessionSnapshot>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT id, thread_title, cwd, source, updated_at, started_at, duration_sec,
                user_message_count, assistant_message_count, tool_call_count, turn_count,
                warning_count, first_user_message, COALESCE(updated_at, started_at) AS ts
         FROM sessions
         WHERE COALESCE(updated_at, started_at) IS NOT NULL
           AND {SESSION_DATE_SQL} >= ?1
           AND {SESSION_DATE_SQL} <= ?2"
    ))?;
    let rows = stmt.query_map(
        params![
            scope.start.format("%Y-%m-%d").to_string(),
            scope.end.format("%Y-%m-%d").to_string()
        ],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, String>(13)?,
            ))
        },
    )?;
    let mut snapshots = Vec::new();

    for row in rows.filter_map(Result::ok) {
        let (
            id,
            thread_title,
            cwd,
            source,
            updated_at,
            started_at,
            duration_sec,
            user_message_count,
            assistant_message_count,
            tool_call_count,
            turn_count,
            warning_count,
            first_user_message,
            value,
        ) = row;
        if !matches_project_filter(cwd.as_deref(), selected_project) {
            continue;
        }
        if let Some(timestamp) = parse_rfc3339_utc(&value) {
            snapshots.push(SessionSnapshot {
                id,
                date: to_local_date(timestamp),
                thread_title,
                cwd,
                source,
                updated_at,
                started_at,
                duration_sec,
                user_message_count,
                assistant_message_count,
                tool_call_count,
                turn_count,
                warning_count,
                first_user_message,
            });
        }
    }

    Ok(snapshots)
}

fn build_daily_activity_map(
    scope: &ResolvedDashboardScope,
    session_snapshots: &[SessionSnapshot],
    turn_analytics: &TurnAnalytics,
) -> BTreeMap<NaiveDate, ActivityAccumulator> {
    let mut timeline = BTreeMap::<NaiveDate, ActivityAccumulator>::new();
    let total_days = (scope.end - scope.start).num_days();

    for offset in 0..=total_days {
        let date = scope.start + Duration::days(offset);
        timeline.insert(date, ActivityAccumulator::default());
    }

    for snapshot in session_snapshots {
        if let Some(entry) = timeline.get_mut(&snapshot.date) {
            entry.sessions += 1;
        }
    }

    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let Some(date) = turn.local_date() else {
            continue;
        };
        if let Some(entry) = timeline.get_mut(&date) {
            entry.questions += 1;
            if let Some(value) = turn.first_response_sec() {
                entry.first_response_total += value;
                entry.first_response_samples += 1;
            }
            if let Some(value) = turn.completion_sec() {
                entry.completion_total += value;
                entry.completion_samples += 1;
            }
        }
    }

    timeline
}

fn activity_points_from_daily_map(
    timeline: &BTreeMap<NaiveDate, ActivityAccumulator>,
) -> Vec<ActivityPoint> {
    timeline
        .iter()
        .map(|(date, item)| ActivityPoint {
            date: date.format("%Y-%m-%d").to_string(),
            sessions: item.sessions,
            questions: item.questions,
            avg_first_response_sec: average_i64(
                item.first_response_total,
                item.first_response_samples,
            ),
            avg_turn_completion_sec: average_i64(item.completion_total, item.completion_samples),
        })
        .collect()
}

fn aggregate_activity(
    timeline: &BTreeMap<NaiveDate, ActivityAccumulator>,
    granularity: TimeGranularity,
) -> Vec<ActivityPoint> {
    if granularity == TimeGranularity::Day {
        return activity_points_from_daily_map(timeline);
    }

    let Some(first_date) = timeline.keys().next().copied() else {
        return Vec::new();
    };
    let Some(last_date) = timeline.keys().last().copied() else {
        return Vec::new();
    };

    let mut buckets = HashMap::<NaiveDate, ActivityAccumulator>::new();
    for (date, item) in timeline {
        let bucket = bucket_start(*date, granularity);
        let entry = buckets.entry(bucket).or_default();
        entry.sessions += item.sessions;
        entry.questions += item.questions;
        entry.first_response_total += item.first_response_total;
        entry.first_response_samples += item.first_response_samples;
        entry.completion_total += item.completion_total;
        entry.completion_samples += item.completion_samples;
    }

    let mut ordered = BTreeMap::<NaiveDate, ActivityAccumulator>::new();
    let mut cursor = bucket_start(first_date, granularity);
    let last_bucket = bucket_start(last_date, granularity);

    while cursor <= last_bucket {
        ordered.insert(cursor, buckets.remove(&cursor).unwrap_or_default());
        cursor = next_bucket_start(cursor, granularity);
    }

    ordered
        .into_iter()
        .map(|(date, item)| ActivityPoint {
            date: format_bucket_label(date, granularity),
            sessions: item.sessions,
            questions: item.questions,
            avg_first_response_sec: average_i64(
                item.first_response_total,
                item.first_response_samples,
            ),
            avg_turn_completion_sec: average_i64(item.completion_total, item.completion_samples),
        })
        .collect()
}

fn build_duration_buckets(
    session_snapshots: &[SessionSnapshot],
) -> anyhow::Result<Vec<ChartDatum>> {
    let mut buckets = vec![
        ("< 5m".to_string(), 0_i64),
        ("5-15m".to_string(), 0_i64),
        ("15-60m".to_string(), 0_i64),
        ("1-3h".to_string(), 0_i64),
        ("> 3h".to_string(), 0_i64),
    ];

    for duration in session_snapshots.iter().map(|item| item.duration_sec) {
        let index = match duration {
            0..=299 => 0,
            300..=899 => 1,
            900..=3599 => 2,
            3600..=10_799 => 3,
            _ => 4,
        };
        buckets[index].1 += 1;
    }

    Ok(buckets
        .into_iter()
        .map(|(label, value)| ChartDatum { label, value })
        .collect())
}

fn build_question_hours(turn_analytics: &TurnAnalytics) -> Vec<ChartDatum> {
    let mut buckets = vec![0_i64; 24];

    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let Some(user_ts) = turn.user_ts else {
            continue;
        };
        let hour = user_ts.with_timezone(&Local).hour() as usize;
        buckets[hour] += 1;
    }

    buckets
        .into_iter()
        .enumerate()
        .map(|(hour, value)| ChartDatum {
            label: format!("{hour:02}"),
            value,
        })
        .collect()
}

fn build_first_response_buckets(turn_analytics: &TurnAnalytics) -> Vec<ChartDatum> {
    let mut buckets = vec![
        ("< 3s".to_string(), 0_i64),
        ("3-10s".to_string(), 0_i64),
        ("10-30s".to_string(), 0_i64),
        ("30-60s".to_string(), 0_i64),
        ("1-3m".to_string(), 0_i64),
        ("> 3m".to_string(), 0_i64),
    ];

    for latency in turn_analytics
        .turns
        .iter()
        .filter_map(TurnRecord::first_response_sec)
    {
        let index = match latency {
            0..=2 => 0,
            3..=10 => 1,
            11..=30 => 2,
            31..=60 => 3,
            61..=180 => 4,
            _ => 5,
        };
        buckets[index].1 += 1;
    }

    buckets
        .into_iter()
        .map(|(label, value)| ChartDatum { label, value })
        .collect()
}

fn build_completion_buckets(turn_analytics: &TurnAnalytics) -> Vec<ChartDatum> {
    let mut buckets = vec![
        ("< 10s".to_string(), 0_i64),
        ("10-30s".to_string(), 0_i64),
        ("30-60s".to_string(), 0_i64),
        ("1-3m".to_string(), 0_i64),
        ("3-10m".to_string(), 0_i64),
        ("> 10m".to_string(), 0_i64),
    ];

    for latency in turn_analytics
        .turns
        .iter()
        .filter_map(TurnRecord::completion_sec)
    {
        let index = match latency {
            0..=9 => 0,
            10..=30 => 1,
            31..=60 => 2,
            61..=180 => 3,
            181..=600 => 4,
            _ => 5,
        };
        buckets[index].1 += 1;
    }

    buckets
        .into_iter()
        .map(|(label, value)| ChartDatum { label, value })
        .collect()
}

fn build_tool_metrics(
    metrics: &HashMap<String, ToolMetricAccumulator>,
    limit: usize,
) -> Vec<ToolMetricDatum> {
    let mut items = metrics
        .iter()
        .map(|(label, metric)| ToolMetricDatum {
            label: label.clone(),
            total: metric.total,
            success: metric.success,
            failure: metric.failure,
            avg_duration_sec: if metric.duration_samples <= 0 {
                0.0
            } else {
                metric.duration_total_sec / metric.duration_samples as f64
            },
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .total
            .cmp(&left.total)
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(limit);
    items
}

fn build_prompt_length_buckets(turn_analytics: &TurnAnalytics) -> Vec<ChartDatum> {
    let mut buckets = vec![
        ("< 40".to_string(), 0_i64),
        ("40-120".to_string(), 0_i64),
        ("120-300".to_string(), 0_i64),
        ("300-800".to_string(), 0_i64),
        ("> 800".to_string(), 0_i64),
    ];

    for length in &turn_analytics.prompt_lengths {
        let index = match *length {
            0..=39 => 0,
            40..=120 => 1,
            121..=300 => 2,
            301..=800 => 3,
            _ => 4,
        };
        buckets[index].1 += 1;
    }

    buckets
        .into_iter()
        .map(|(label, value)| ChartDatum { label, value })
        .collect()
}

fn build_prompt_composition(turn_analytics: &TurnAnalytics) -> Vec<ChartDatum> {
    let total_questions = turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
        .count() as i64;
    let percent = |value: i64| -> i64 {
        if total_questions <= 0 {
            0
        } else {
            ((value as f64 / total_questions as f64) * 100.0).round() as i64
        }
    };

    vec![
        ChartDatum {
            label: "With code".to_string(),
            value: percent(turn_analytics.prompts_with_code),
        },
        ChartDatum {
            label: "With path".to_string(),
            value: percent(turn_analytics.prompts_with_path),
        },
        ChartDatum {
            label: "With command".to_string(),
            value: percent(turn_analytics.prompts_with_command),
        },
        ChartDatum {
            label: "Path or command".to_string(),
            value: percent(turn_analytics.prompts_with_path_or_command),
        },
    ]
}

fn build_transport_signals(log_analytics: &LogAnalytics) -> Vec<ChartDatum> {
    vec![
        ChartDatum {
            label: "Reconnect".to_string(),
            value: *log_analytics.signals.get("Reconnect").unwrap_or(&0),
        },
        ChartDatum {
            label: "Retry".to_string(),
            value: *log_analytics.signals.get("Retry").unwrap_or(&0),
        },
        ChartDatum {
            label: "Transport error".to_string(),
            value: *log_analytics.signals.get("Transport error").unwrap_or(&0),
        },
    ]
}

fn build_workspace_switches(turn_analytics: &TurnAnalytics) -> Vec<ChartDatum> {
    vec![
        ChartDatum {
            label: "Project switch".to_string(),
            value: turn_analytics.project_switches,
        },
        ChartDatum {
            label: "Workspace switch".to_string(),
            value: turn_analytics.workspace_switches,
        },
    ]
}

fn build_search_hours(turn_analytics: &TurnAnalytics) -> Vec<ChartDatum> {
    turn_analytics
        .search_hours
        .iter()
        .enumerate()
        .map(|(hour, value)| ChartDatum {
            label: format!("{hour:02}"),
            value: *value,
        })
        .collect()
}

fn build_hourly_correlations(turn_analytics: &TurnAnalytics) -> Vec<CorrelationDatum> {
    let mut buckets = BTreeMap::<String, CorrelationAccumulator>::new();
    for hour in 0..24 {
        buckets.insert(format!("{hour:02}"), CorrelationAccumulator::default());
    }
    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let hour = turn
            .user_ts
            .map(|value| value.with_timezone(&Local).hour())
            .unwrap_or_default();
        let key = format!("{hour:02}");
        update_correlation(buckets.entry(key).or_default(), turn);
    }
    buckets
        .into_iter()
        .map(|(bucket, item)| correlation_datum(bucket, item))
        .collect()
}

fn build_weekday_correlations(turn_analytics: &TurnAnalytics) -> Vec<CorrelationDatum> {
    let mut weekday = CorrelationAccumulator::default();
    let mut weekend = CorrelationAccumulator::default();
    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let is_weekend = turn
            .user_ts
            .map(|value| {
                matches!(
                    value.with_timezone(&Local).weekday().number_from_monday(),
                    6 | 7
                )
            })
            .unwrap_or(false);
        if is_weekend {
            update_correlation(&mut weekend, turn);
        } else {
            update_correlation(&mut weekday, turn);
        }
    }
    vec![
        correlation_datum("工作日".to_string(), weekday),
        correlation_datum("周末".to_string(), weekend),
    ]
}

fn build_prompt_length_correlations(turn_analytics: &TurnAnalytics) -> Vec<CorrelationDatum> {
    let mut map = ordered_correlation_map(&["<80", "80-200", "200-500", "500-1000", ">1000"]);
    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let bucket = match turn.prompt_length() {
            0..=79 => "<80",
            80..=200 => "80-200",
            201..=500 => "200-500",
            501..=1000 => "500-1000",
            _ => ">1000",
        };
        update_correlation(map.entry(bucket.to_string()).or_default(), turn);
    }
    map.into_iter()
        .map(|(bucket, item)| correlation_datum(bucket, item))
        .collect()
}

fn build_tool_load_correlations(turn_analytics: &TurnAnalytics) -> Vec<CorrelationDatum> {
    let mut map = ordered_correlation_map(&["0", "1", "2", "3+"]);
    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let bucket = match turn.tool_call_count {
            i64::MIN..=0 => "0",
            1 => "1",
            2 => "2",
            _ => "3+",
        };
        update_correlation(map.entry(bucket.to_string()).or_default(), turn);
    }
    map.into_iter()
        .map(|(bucket, item)| correlation_datum(bucket, item))
        .collect()
}

fn build_context_load_correlations(turn_analytics: &TurnAnalytics) -> Vec<CorrelationDatum> {
    let mut map = ordered_correlation_map(&["1", "2", "3", "4+"]);
    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let bucket = match turn.turn_context_count {
            i64::MIN..=1 => "1",
            2 => "2",
            3 => "3",
            _ => "4+",
        };
        update_correlation(map.entry(bucket.to_string()).or_default(), turn);
    }
    map.into_iter()
        .map(|(bucket, item)| correlation_datum(bucket, item))
        .collect()
}

fn ordered_correlation_map(labels: &[&str]) -> BTreeMap<String, CorrelationAccumulator> {
    let mut map = BTreeMap::new();
    for label in labels {
        map.insert((*label).to_string(), CorrelationAccumulator::default());
    }
    map
}

fn update_correlation(acc: &mut CorrelationAccumulator, turn: &TurnRecord) {
    acc.sample_count += 1;
    acc.total_tokens += turn.total_tokens;
    if let Some(value) = turn.first_response_sec() {
        acc.first_response_total += value as f64;
        acc.first_response_samples += 1;
    }
    if let Some(value) = turn.completion_sec() {
        acc.completion_total += value as f64;
        acc.completion_samples += 1;
    }
    if let Some(value) = turn.token_rate() {
        acc.token_rate_total += value;
        acc.token_rate_samples += 1;
    }
}

fn correlation_datum(bucket: String, acc: CorrelationAccumulator) -> CorrelationDatum {
    CorrelationDatum {
        bucket,
        sample_count: acc.sample_count,
        avg_first_response_sec: average_f64(acc.first_response_total, acc.first_response_samples),
        avg_completion_sec: average_f64(acc.completion_total, acc.completion_samples),
        avg_total_tokens: average_f64(acc.total_tokens as f64, acc.sample_count),
        avg_token_rate: average_f64(acc.token_rate_total, acc.token_rate_samples),
    }
}

fn build_correlation_scatter(
    turn_analytics: &TurnAnalytics,
    dimension: ScatterDimension,
) -> Vec<ScatterDatum> {
    let mut points = turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
        .filter_map(|turn| {
            let completion = turn.completion_sec()? as f64;
            let first_response = turn.first_response_sec().unwrap_or_default() as f64;
            let total_tokens = turn.total_tokens.max(0) as f64;
            let token_rate = turn.token_rate().unwrap_or_default();
            let x = match dimension {
                ScatterDimension::HourOfDay => turn
                    .user_ts
                    .map(|value| value.with_timezone(&Local).hour() as f64)
                    .unwrap_or(0.0),
                ScatterDimension::WeekendFlag => turn
                    .user_ts
                    .map(|value| {
                        if matches!(
                            value.with_timezone(&Local).weekday().number_from_monday(),
                            6 | 7
                        ) {
                            1.0
                        } else {
                            0.0
                        }
                    })
                    .unwrap_or(0.0),
                ScatterDimension::PromptLength => turn.prompt_length() as f64,
                ScatterDimension::ToolLoad => turn.tool_call_count as f64,
                ScatterDimension::ContextLoad => turn.turn_context_count as f64,
            };
            let label = turn
                .cwd
                .as_deref()
                .map(project_label_from_cwd)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| turn.session_id.chars().take(12).collect::<String>());
            Some(ScatterDatum {
                x,
                completion_sec: completion,
                total_tokens,
                first_response_sec: first_response,
                token_rate,
                label,
                detail: turn.user_text.clone().unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();

    const MAX_POINTS: usize = 500;
    if points.len() > MAX_POINTS {
        let step = ((points.len() as f64) / (MAX_POINTS as f64)).ceil() as usize;
        points = points
            .into_iter()
            .enumerate()
            .filter(|(index, _)| index % step == 0)
            .map(|(_, item)| item)
            .take(MAX_POINTS)
            .collect();
    }

    points
}

fn build_token_usage_summary(turn_analytics: &TurnAnalytics) -> TokenUsageSummary {
    let mut summary = TokenUsageSummary::default();
    for turn in &turn_analytics.turns {
        summary.input_tokens += turn.input_tokens;
        summary.output_tokens += turn.output_tokens;
        summary.cached_input_tokens += turn.cached_input_tokens;
        summary.reasoning_output_tokens += turn.reasoning_output_tokens;
        summary.total_tokens += turn.total_tokens;
    }
    summary
}

enum RankedTurnMode {
    ByTokens,
    ByCompletionLatency,
}

fn build_ranked_turns(
    turn_analytics: &TurnAnalytics,
    sessions: &[SessionSnapshot],
    mode: RankedTurnMode,
    limit: usize,
) -> Vec<RankedTurnRecord> {
    let session_meta = sessions
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    let mut items = turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
        .map(|turn| {
            let meta = session_meta.get(&turn.session_id);
            RankedTurnRecord {
                session_id: turn.session_id.clone(),
                project: project_label_from_option(
                    turn.cwd
                        .as_deref()
                        .or_else(|| meta.and_then(|item| item.cwd.as_deref())),
                ),
                thread_title: meta.and_then(|item| item.thread_title.clone()),
                prompt_preview: turn.user_text.clone(),
                total_tokens: turn.total_tokens,
                input_tokens: turn.input_tokens,
                output_tokens: turn.output_tokens,
                cached_input_tokens: turn.cached_input_tokens,
                reasoning_output_tokens: turn.reasoning_output_tokens,
                first_response_sec: turn.first_response_sec(),
                completion_sec: turn.completion_sec(),
                timestamp: turn
                    .user_ts
                    .or(turn.started_at)
                    .map(|value| value.to_rfc3339()),
            }
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        let left_score = match mode {
            RankedTurnMode::ByTokens => left.total_tokens,
            RankedTurnMode::ByCompletionLatency => left.completion_sec.unwrap_or_default(),
        };
        let right_score = match mode {
            RankedTurnMode::ByTokens => right.total_tokens,
            RankedTurnMode::ByCompletionLatency => right.completion_sec.unwrap_or_default(),
        };
        right_score
            .cmp(&left_score)
            .then_with(|| right.total_tokens.cmp(&left.total_tokens))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    items.retain(|item| match mode {
        RankedTurnMode::ByTokens => item.total_tokens > 0,
        RankedTurnMode::ByCompletionLatency => item.completion_sec.unwrap_or_default() > 0,
    });
    items.truncate(limit);
    items
}

fn build_project_timeline(
    scope: &ResolvedDashboardScope,
    sessions: &[SessionSnapshot],
    turn_analytics: &TurnAnalytics,
) -> Vec<BreakdownDatum> {
    let session_meta = sessions
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    let mut project_counts = HashMap::<String, i64>::new();
    for session in sessions {
        let label = project_label_from_option(session.cwd.as_deref());
        *project_counts.entry(label).or_default() += 1;
    }
    let top_projects = top_labels(&project_counts, 6);
    let mut buckets = BTreeMap::<String, HashMap<String, i64>>::new();
    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let Some(anchor_ts) = turn.anchor_ts() else {
            continue;
        };
        let bucket = bucket_label_for_timestamp(anchor_ts, scope.granularity);
        let label = project_label_from_option(turn.cwd.as_deref().or_else(|| {
            session_meta
                .get(&turn.session_id)
                .and_then(|item| item.cwd.as_deref())
        }));
        if top_projects.contains(&label) {
            increment_breakdown(&mut buckets, bucket, &label, 1);
        }
    }
    build_dense_breakdown_series(&buckets, &top_projects, scope)
}

fn build_project_summaries(
    sessions: &[SessionSnapshot],
    turn_analytics: &TurnAnalytics,
    limit: usize,
) -> Vec<ProjectSummary> {
    let session_meta = sessions
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    #[derive(Default)]
    struct ProjectAccumulator {
        session_count: i64,
        question_count: i64,
        total_tokens: i64,
        context_compactions: i64,
        first_response_total: i64,
        first_response_samples: i64,
        completion_total: i64,
        completion_samples: i64,
        max_parallel_windows: i64,
    }

    let mut acc = HashMap::<String, ProjectAccumulator>::new();
    for session in sessions {
        let label = project_label_from_option(session.cwd.as_deref());
        acc.entry(label).or_default().session_count += 1;
    }
    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let label = project_label_from_option(turn.cwd.as_deref().or_else(|| {
            session_meta
                .get(&turn.session_id)
                .and_then(|item| item.cwd.as_deref())
        }));
        let entry = acc.entry(label).or_default();
        entry.question_count += 1;
        entry.total_tokens += turn.total_tokens;
        if let Some(value) = turn.first_response_sec() {
            entry.first_response_total += value;
            entry.first_response_samples += 1;
        }
        if let Some(value) = turn.completion_sec() {
            entry.completion_total += value;
            entry.completion_samples += 1;
        }
    }
    for (project, value) in &turn_analytics.project_compactions {
        acc.entry(project.clone()).or_default().context_compactions += *value;
    }
    for item in build_project_parallelism(sessions, limit.saturating_mul(2)) {
        if let Some(entry) = acc.get_mut(&item.label) {
            entry.max_parallel_windows = item.value;
        }
    }

    let mut items = acc
        .into_iter()
        .map(|(label, item)| ProjectSummary {
            label,
            session_count: item.session_count,
            question_count: item.question_count,
            total_tokens: item.total_tokens,
            context_compactions: item.context_compactions,
            avg_first_response_sec: if item.first_response_samples <= 0 {
                0.0
            } else {
                item.first_response_total as f64 / item.first_response_samples as f64
            },
            avg_completion_sec: if item.completion_samples <= 0 {
                0.0
            } else {
                item.completion_total as f64 / item.completion_samples as f64
            },
            max_parallel_windows: item.max_parallel_windows,
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .total_tokens
            .cmp(&left.total_tokens)
            .then_with(|| right.session_count.cmp(&left.session_count))
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(limit);
    items
}

fn build_project_windows(
    sessions: &[SessionSnapshot],
    turn_analytics: &TurnAnalytics,
    limit: usize,
) -> Vec<ProjectWindowRecord> {
    let mut turn_totals = HashMap::<String, (i64, i64)>::new();
    for turn in turn_analytics
        .turns
        .iter()
        .filter(|turn| turn.user_ts.is_some())
    {
        let entry = turn_totals.entry(turn.session_id.clone()).or_default();
        entry.0 += turn.total_tokens;
        entry.1 += 1;
    }
    let mut items = sessions
        .iter()
        .map(|session| {
            let (total_tokens, question_count) =
                turn_totals.get(&session.id).copied().unwrap_or_default();
            ProjectWindowRecord {
                session_id: session.id.clone(),
                project: project_label_from_option(session.cwd.as_deref()),
                thread_title: session.thread_title.clone(),
                started_at: session.started_at.clone(),
                updated_at: session.updated_at.clone(),
                duration_sec: session.duration_sec,
                turn_count: session.turn_count,
                total_tokens,
                question_count,
                tool_call_count: session.tool_call_count,
            }
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .total_tokens
            .cmp(&left.total_tokens)
            .then_with(|| right.duration_sec.cmp(&left.duration_sec))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    items.truncate(limit);
    items
}

fn build_project_parallelism(sessions: &[SessionSnapshot], limit: usize) -> Vec<ChartDatum> {
    let mut ranges = HashMap::<String, Vec<(DateTime<Utc>, i32)>>::new();
    for session in sessions {
        let Some(start) = session.started_at.as_deref().and_then(parse_rfc3339_utc) else {
            continue;
        };
        let Some(end) = session
            .updated_at
            .as_deref()
            .and_then(parse_rfc3339_utc)
            .or_else(|| Some(start + Duration::seconds(session.duration_sec.max(0))))
        else {
            continue;
        };
        let label = project_label_from_option(session.cwd.as_deref());
        let entry = ranges.entry(label).or_default();
        entry.push((start, 1));
        entry.push((end, -1));
    }
    let mut items = Vec::new();
    for (label, mut markers) in ranges {
        markers.sort_by_key(|item| item.0);
        let mut active = 0_i64;
        let mut peak = 0_i64;
        for (_, delta) in markers {
            active += delta as i64;
            peak = peak.max(active);
        }
        items.push(ChartDatum { label, value: peak });
    }
    items.sort_by(|left, right| {
        right
            .value
            .cmp(&left.value)
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(limit);
    items
}

fn build_dense_breakdown_series(
    timeline: &BTreeMap<String, HashMap<String, i64>>,
    categories: &[String],
    scope: &ResolvedDashboardScope,
) -> Vec<BreakdownDatum> {
    let mut rows = Vec::new();
    for bucket in bucket_labels_in_scope(scope) {
        for category in categories {
            rows.push(BreakdownDatum {
                bucket: bucket.clone(),
                category: category.clone(),
                value: timeline
                    .get(&bucket)
                    .and_then(|values| values.get(category))
                    .copied()
                    .unwrap_or(0),
            });
        }
    }
    rows
}

fn bucket_labels_in_scope(scope: &ResolvedDashboardScope) -> Vec<String> {
    let mut labels = Vec::new();
    let mut cursor = bucket_start(scope.start, scope.granularity);
    let last = bucket_start(scope.end, scope.granularity);
    while cursor <= last {
        labels.push(format_bucket_label(cursor, scope.granularity));
        cursor = next_bucket_start(cursor, scope.granularity);
    }
    labels
}

fn top_metrics_from_sessions(
    sessions: &[SessionSnapshot],
    selector: impl Fn(&SessionSnapshot) -> Option<String>,
    limit: usize,
) -> Vec<ChartDatum> {
    let mut counts = HashMap::<String, i64>::new();
    for session in sessions {
        let label = selector(session)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "Unknown".to_string());
        *counts.entry(label).or_default() += 1;
    }
    top_chart_data(&counts, limit)
}

fn recent_sessions_from_snapshots(
    sessions: &[SessionSnapshot],
    limit: usize,
) -> Vec<SessionSummary> {
    let mut items = sessions.to_vec();
    items.sort_by(|left, right| {
        right
            .updated_at
            .as_deref()
            .or(right.started_at.as_deref())
            .cmp(&left.updated_at.as_deref().or(left.started_at.as_deref()))
    });
    items
        .into_iter()
        .take(limit)
        .map(session_snapshot_to_summary)
        .collect()
}

fn load_recent_imports(conn: &Connection) -> anyhow::Result<Vec<RecentImport>> {
    let mut stmt = conn.prepare(
        "SELECT imports.id, data_sources.label, status, mode, files_total, files_success, files_failed,
                warnings_count, errors_count, started_at, finished_at
         FROM imports
         JOIN data_sources ON data_sources.id = imports.source_id
         ORDER BY started_at DESC
         LIMIT 8",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RecentImport {
            id: row.get(0)?,
            source_label: row.get(1)?,
            status: row.get(2)?,
            mode: row.get(3)?,
            files_total: row.get(4)?,
            files_success: row.get(5)?,
            files_failed: row.get(6)?,
            warnings_count: row.get(7)?,
            errors_count: row.get(8)?,
            started_at: row.get(9)?,
            finished_at: row.get(10)?,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn load_recent_issues(conn: &Connection) -> anyhow::Result<Vec<ImportIssueRecord>> {
    let mut stmt = conn.prepare(
        "SELECT import_issues.id,
                import_issues.severity,
                import_issues.code,
                import_issues.message,
                import_issues.line_no,
                import_issues.raw_excerpt,
                import_issues.created_at,
                source_files.abs_path
         FROM import_issues
         LEFT JOIN source_files ON source_files.id = import_issues.source_file_id
         ORDER BY import_issues.created_at DESC
         LIMIT 8",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ImportIssueRecord {
            id: row.get(0)?,
            severity: row.get(1)?,
            code: row.get(2)?,
            message: row.get(3)?,
            line_no: row.get(4)?,
            raw_excerpt: row.get(5)?,
            created_at: row.get(6)?,
            path: row.get(7)?,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn fetch_session_summary(conn: &Connection, session_id: &str) -> anyhow::Result<SessionSummary> {
    Ok(conn.query_row(
        "SELECT id, thread_title, cwd, source, updated_at, started_at, duration_sec, user_message_count,
                assistant_message_count, tool_call_count, turn_count, warning_count, first_user_message
         FROM sessions
         WHERE id = ?1",
        params![session_id],
        map_session_summary,
    )?)
}

fn fetch_session_messages(
    conn: &Connection,
    session_id: &str,
) -> anyhow::Result<Vec<SessionMessageRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, turn_id, role, kind, text, ts, tool_name, phase, meta_json
         FROM session_messages
         WHERE session_id = ?1
         ORDER BY COALESCE(ts, ''), rowid",
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        let meta_json: String = row.get(8)?;
        Ok(SessionMessageRecord {
            id: row.get(0)?,
            turn_id: row.get(1)?,
            role: row.get(2)?,
            kind: row.get(3)?,
            text: row.get(4)?,
            ts: row.get(5)?,
            tool_name: row.get(6)?,
            phase: row.get(7)?,
            image_urls: extract_message_image_urls(&meta_json),
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn extract_message_image_urls(meta_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<JsonValue>(meta_json) else {
        return Vec::new();
    };

    let mut urls = Vec::new();
    collect_image_urls(&value, &mut urls);
    urls
}

fn collect_image_urls(value: &JsonValue, urls: &mut Vec<String>) {
    match value {
        JsonValue::String(text) if text.starts_with("data:image/") => {
            if !urls.iter().any(|url| url == text) {
                urls.push(text.clone());
            }
        }
        JsonValue::Array(items) => {
            for item in items {
                collect_image_urls(item, urls);
            }
        }
        JsonValue::Object(map) => {
            for value in map.values() {
                collect_image_urls(value, urls);
            }
        }
        _ => {}
    }
}

fn map_session_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionSummary> {
    Ok(SessionSummary {
        id: row.get(0)?,
        thread_title: row.get(1)?,
        cwd: row.get(2)?,
        source: row.get(3)?,
        updated_at: row.get(4)?,
        started_at: row.get(5)?,
        duration_sec: row.get(6)?,
        user_message_count: row.get(7)?,
        assistant_message_count: row.get(8)?,
        tool_call_count: row.get(9)?,
        turn_count: row.get(10)?,
        warning_count: row.get(11)?,
        first_user_message: row.get(12)?,
    })
}

fn session_snapshot_to_summary(item: SessionSnapshot) -> SessionSummary {
    SessionSummary {
        id: item.id,
        thread_title: item.thread_title,
        cwd: item.cwd,
        source: item.source,
        updated_at: item.updated_at,
        started_at: item.started_at,
        duration_sec: item.duration_sec,
        user_message_count: item.user_message_count,
        assistant_message_count: item.assistant_message_count,
        tool_call_count: item.tool_call_count,
        turn_count: item.turn_count,
        warning_count: item.warning_count,
        first_user_message: item.first_user_message,
    }
}

fn build_turn_analytics(
    conn: &Connection,
    scope: &ResolvedDashboardScope,
    selected_project: Option<&str>,
    sessions: &[SessionSnapshot],
) -> anyhow::Result<TurnAnalytics> {
    let mut stmt = conn.prepare(
        "SELECT session_id, seq, ts, outer_type, inner_type, payload_json
         FROM session_events_raw
         ORDER BY session_id, seq",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;

    let mut current_session_id = String::new();
    let mut current_turn_id: Option<String> = None;
    let mut turns = HashMap::<String, TurnRecord>::new();
    let mut all_turns = Vec::<TurnRecord>::new();
    let mut context_compactions = 0_i64;
    let mut rolled_back_turns = 0_i64;
    let mut interruption_timeline = BTreeMap::<String, HashMap<String, i64>>::new();
    let mut tool_types = HashMap::<String, i64>::new();
    let mut tool_metrics = HashMap::<String, ToolMetricAccumulator>::new();
    let mut search_terms = HashMap::<String, i64>::new();
    let mut search_hours = [0_i64; 24];
    let mut pending_calls = HashMap::<String, String>::new();
    let mut project_compactions = HashMap::<String, i64>::new();
    let session_cwds = sessions
        .iter()
        .map(|item| (item.id.clone(), item.cwd.clone()))
        .collect::<HashMap<_, _>>();
    let mut current_session_project: Option<String> = None;

    for row in rows.filter_map(Result::ok) {
        let (session_id, _seq, ts, outer_type, inner_type, payload_json) = row;
        if current_session_id != session_id {
            flush_turns(&mut turns, &mut all_turns);
            current_session_id = session_id.clone();
            current_turn_id = None;
            current_session_project = session_cwds
                .get(&session_id)
                .and_then(|value| value.as_deref())
                .map(project_label_from_cwd);
        }

        let payload = serde_json::from_str::<JsonValue>(&payload_json).unwrap_or(JsonValue::Null);
        let timestamp = ts.as_deref().and_then(parse_rfc3339_utc);

        match (outer_type.as_str(), inner_type.as_deref()) {
            ("event_msg", Some("task_started")) => {
                if let Some(turn_id) = payload.get("turn_id").and_then(JsonValue::as_str) {
                    current_turn_id = Some(turn_id.to_string());
                    let turn = ensure_turn(&mut turns, turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    turn.started_at = payload
                        .get("started_at")
                        .and_then(JsonValue::as_i64)
                        .and_then(unix_seconds_to_utc)
                        .or(timestamp);
                }
            }
            ("turn_context", _) => {
                if let Some(turn_id) = payload.get("turn_id").and_then(JsonValue::as_str) {
                    current_turn_id = Some(turn_id.to_string());
                    let turn = ensure_turn(&mut turns, turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    turn.turn_context_count += 1;
                    turn.model = payload
                        .get("model")
                        .and_then(JsonValue::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| turn.model.clone());
                    turn.effort = payload
                        .get("effort")
                        .and_then(JsonValue::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            payload
                                .get("collaboration_mode")
                                .and_then(|value| value.get("settings"))
                                .and_then(|value| value.get("reasoning_effort"))
                                .and_then(JsonValue::as_str)
                                .map(ToOwned::to_owned)
                        })
                        .or_else(|| turn.effort.clone());
                    turn.cwd = payload
                        .get("cwd")
                        .and_then(JsonValue::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| turn.cwd.clone());
                    if let Some(cwd) = turn.cwd.as_deref() {
                        current_session_project = Some(project_label_from_cwd(cwd));
                    }
                }
            }
            ("event_msg", Some("user_message")) => {
                if let Some(turn_id) = resolve_turn_id(&payload, current_turn_id.as_deref()) {
                    let turn = ensure_turn(&mut turns, &turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    turn.user_ts = turn.user_ts.or(timestamp);
                    if turn.user_text.is_none() {
                        turn.user_text = payload
                            .get("message")
                            .and_then(JsonValue::as_str)
                            .map(|value| value.trim().to_string());
                    }
                }
            }
            ("event_msg", Some("agent_reasoning")) => {
                if let Some(turn_id) = current_turn_id.as_deref() {
                    let turn = ensure_turn(&mut turns, turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    if turn.first_assistant_ts.is_none() {
                        turn.first_assistant_ts = timestamp;
                    }
                }
            }
            ("event_msg", Some("agent_message")) => {
                if let Some(turn_id) = current_turn_id.as_deref() {
                    let turn = ensure_turn(&mut turns, turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    if turn.first_assistant_ts.is_none() {
                        turn.first_assistant_ts = timestamp;
                    }
                }
            }
            ("response_item", Some("function_call"))
            | ("response_item", Some("custom_tool_call"))
            | ("response_item", Some("web_search_call")) => {
                let label = tool_label(inner_type.as_deref(), &payload);
                *tool_types.entry(label.clone()).or_default() += 1;
                tool_metrics.entry(label.clone()).or_default().total += 1;

                if let Some(call_id) = payload.get("call_id").and_then(JsonValue::as_str) {
                    pending_calls.insert(call_id.to_string(), label.clone());
                }

                if matches!(inner_type.as_deref(), Some("web_search_call")) {
                    if let Some(event_ts) =
                        timestamp.filter(|value| scope.contains_date(to_local_date(*value)))
                    {
                        search_hours[event_ts.with_timezone(&Local).hour() as usize] += 1;
                    }
                    for query in extract_search_queries(&payload) {
                        update_prompt_terms(&mut search_terms, &query);
                    }
                    tool_metrics.entry(label).or_default().success += 1;
                }

                if let Some(turn_id) = resolve_turn_id(&payload, current_turn_id.as_deref()) {
                    let turn = ensure_turn(&mut turns, &turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    turn.tool_call_count += 1;
                }
            }
            ("event_msg", Some("view_image_tool_call")) => {
                *tool_types.entry("view_image".to_string()).or_default() += 1;
                let metric = tool_metrics.entry("view_image".to_string()).or_default();
                metric.total += 1;
                metric.success += 1;
                if let Some(turn_id) = current_turn_id.as_deref() {
                    let turn = ensure_turn(&mut turns, turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    turn.tool_call_count += 1;
                }
            }
            ("event_msg", Some("token_count")) | ("token_count", _) => {
                if let Some(turn_id) = current_turn_id.as_deref() {
                    let turn = ensure_turn(&mut turns, turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    if let Some(usage) = extract_last_token_usage(&payload) {
                        let signature = format!(
                            "{}:{}:{}:{}:{}",
                            usage.cached_input_tokens,
                            usage.input_tokens,
                            usage.output_tokens,
                            usage.reasoning_output_tokens,
                            usage.total_tokens
                        );
                        if turn.last_token_signature.as_deref() != Some(signature.as_str()) {
                            turn.cached_input_tokens += usage.cached_input_tokens;
                            turn.input_tokens += usage.input_tokens;
                            turn.output_tokens += usage.output_tokens;
                            turn.reasoning_output_tokens += usage.reasoning_output_tokens;
                            turn.total_tokens += usage.total_tokens;
                            turn.last_token_signature = Some(signature);
                        }
                    }
                }
            }
            ("response_item", Some("function_call_output")) => {
                let label = payload
                    .get("call_id")
                    .and_then(JsonValue::as_str)
                    .and_then(|call_id| pending_calls.remove(call_id))
                    .unwrap_or_else(|| "tool_call".to_string());
                let metric = tool_metrics.entry(label).or_default();
                let output = payload
                    .get("output")
                    .and_then(JsonValue::as_str)
                    .unwrap_or_default();
                let exit_code = parse_function_call_exit_code(output);
                let duration = parse_wall_time_seconds(output);
                apply_tool_result(metric, exit_code, duration);
            }
            ("response_item", Some("custom_tool_call_output")) => {
                let label = payload
                    .get("call_id")
                    .and_then(JsonValue::as_str)
                    .and_then(|call_id| pending_calls.remove(call_id))
                    .unwrap_or_else(|| "custom_tool".to_string());
                let metric = tool_metrics.entry(label).or_default();
                let (exit_code, duration) = parse_custom_tool_output(payload.get("output"));
                apply_tool_result(metric, exit_code, duration);
            }
            ("event_msg", Some("task_complete")) => {
                if let Some(turn_id) = resolve_turn_id(&payload, current_turn_id.as_deref()) {
                    let turn = ensure_turn(&mut turns, &turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    turn.completed_ts = payload
                        .get("completed_at")
                        .and_then(JsonValue::as_i64)
                        .and_then(unix_seconds_to_utc)
                        .or(timestamp);
                    turn.task_duration_ms = payload
                        .get("duration_ms")
                        .and_then(JsonValue::as_i64)
                        .or(turn.task_duration_ms);
                }
            }
            ("event_msg", Some("turn_aborted")) => {
                if let Some(turn_id) = resolve_turn_id(&payload, current_turn_id.as_deref()) {
                    let turn = ensure_turn(&mut turns, &turn_id);
                    if turn.session_id.is_empty() {
                        turn.session_id = session_id.clone();
                    }
                    turn.aborted = true;
                }
                if let Some(event_ts) =
                    timestamp.filter(|value| scope.contains_date(to_local_date(*value)))
                {
                    let bucket = bucket_label_for_timestamp(event_ts, scope.granularity);
                    increment_breakdown(&mut interruption_timeline, bucket, "Turn aborted", 1);
                }
            }
            ("event_msg", Some("thread_rolled_back")) => {
                if let Some(event_ts) =
                    timestamp.filter(|value| scope.contains_date(to_local_date(*value)))
                {
                    rolled_back_turns += 1;
                    let bucket = bucket_label_for_timestamp(event_ts, scope.granularity);
                    increment_breakdown(&mut interruption_timeline, bucket, "Rollback", 1);
                }
            }
            ("event_msg", Some("context_compacted")) | ("compacted", _) => {
                if let Some(event_ts) = timestamp {
                    if scope.contains_date(to_local_date(event_ts)) {
                        context_compactions += 1;
                        if let Some(project) = current_session_project.clone() {
                            *project_compactions.entry(project).or_default() += 1;
                        }
                        let bucket = bucket_label_for_timestamp(event_ts, scope.granularity);
                        increment_breakdown(
                            &mut interruption_timeline,
                            bucket,
                            "Context compacted",
                            1,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    flush_turns(&mut turns, &mut all_turns);
    Ok(summarize_turns(
        all_turns,
        scope,
        context_compactions,
        rolled_back_turns,
        interruption_timeline,
        tool_types,
        tool_metrics,
        search_terms,
        search_hours,
        project_compactions,
        selected_project,
        &session_cwds,
    )?)
}

fn flush_turns(turns: &mut HashMap<String, TurnRecord>, all_turns: &mut Vec<TurnRecord>) {
    for (_, turn) in turns.drain() {
        all_turns.push(turn);
    }
}

fn summarize_turns(
    all_turns: Vec<TurnRecord>,
    scope: &ResolvedDashboardScope,
    context_compactions: i64,
    rolled_back_turns: i64,
    interruption_timeline: BTreeMap<String, HashMap<String, i64>>,
    tool_types: HashMap<String, i64>,
    tool_metrics: HashMap<String, ToolMetricAccumulator>,
    search_terms: HashMap<String, i64>,
    search_hours: [i64; 24],
    project_compactions: HashMap<String, i64>,
    selected_project: Option<&str>,
    session_cwds: &HashMap<String, Option<String>>,
) -> anyhow::Result<TurnAnalytics> {
    let mut analytics = TurnAnalytics {
        context_compactions,
        rolled_back_turns,
        interruption_timeline,
        tool_types,
        tool_metrics,
        search_terms,
        search_hours,
        project_compactions,
        ..Default::default()
    };
    let mut filtered_turns = all_turns
        .into_iter()
        .filter(|turn| {
            turn.local_date()
                .is_some_and(|date| scope.contains_date(date))
        })
        .filter(|turn| {
            let cwd = turn.cwd.as_deref().or_else(|| {
                session_cwds
                    .get(&turn.session_id)
                    .and_then(|value| value.as_deref())
            });
            matches_project_filter(cwd, selected_project)
        })
        .collect::<Vec<_>>();

    filtered_turns.sort_by_key(|turn| turn.anchor_ts());

    let mut previous_cwd: Option<String> = None;
    let mut previous_project: Option<String> = None;
    for turn in filtered_turns {
        let Some(anchor_ts) = turn.anchor_ts() else {
            continue;
        };
        let bucket = bucket_label_for_timestamp(anchor_ts, scope.granularity);
        let effective_cwd = turn
            .cwd
            .clone()
            .or_else(|| session_cwds.get(&turn.session_id).cloned().flatten());
        if turn.turn_context_count > 1 {
            analytics.repeated_turn_contexts += turn.turn_context_count - 1;
        }
        if turn.aborted {
            analytics.aborted_turns += 1;
        }
        if let Some(model) = turn.model.clone().filter(|value| !value.trim().is_empty()) {
            *analytics.models.entry(model).or_default() += 1;
            increment_breakdown(
                &mut analytics.model_timeline,
                bucket.clone(),
                turn.model.as_deref().unwrap_or("Unknown"),
                1,
            );
        }
        if let Some(effort) = turn.effort.clone().filter(|value| !value.trim().is_empty()) {
            *analytics.efforts.entry(effort).or_default() += 1;
            increment_breakdown(
                &mut analytics.effort_timeline,
                bucket.clone(),
                turn.effort.as_deref().unwrap_or("Unknown"),
                1,
            );
        }
        if let Some(text) = turn.user_text.as_deref() {
            update_prompt_terms(&mut analytics.prompt_terms, text);
            analytics.prompt_lengths.push(text.chars().count());
            let has_code = contains_code_hint(text);
            let has_path = contains_path_hint(text);
            let has_command = contains_command_hint(text);
            if has_code {
                analytics.prompts_with_code += 1;
            }
            if has_path {
                analytics.prompts_with_path += 1;
            }
            if has_command {
                analytics.prompts_with_command += 1;
            }
            if has_path || has_command {
                analytics.prompts_with_path_or_command += 1;
            }
        }
        if let Some(current_cwd) = effective_cwd.filter(|value| !value.trim().is_empty()) {
            let normalized = normalize_workspace(&current_cwd);
            let current_project = project_key(&normalized);
            if previous_cwd
                .as_deref()
                .is_some_and(|previous| previous != normalized)
            {
                analytics.workspace_switches += 1;
                increment_breakdown(
                    &mut analytics.workspace_timeline,
                    bucket.clone(),
                    "Workspace switch",
                    1,
                );
            }
            if previous_project
                .as_deref()
                .is_some_and(|previous| previous != current_project)
            {
                analytics.project_switches += 1;
                increment_breakdown(
                    &mut analytics.workspace_timeline,
                    bucket.clone(),
                    "Project switch",
                    1,
                );
            }
            previous_cwd = Some(normalized);
            previous_project = Some(current_project);
        }
        analytics.turns.push(turn);
    }

    Ok(analytics)
}

fn ensure_turn<'a>(
    turns: &'a mut HashMap<String, TurnRecord>,
    turn_id: &str,
) -> &'a mut TurnRecord {
    turns.entry(turn_id.to_string()).or_default()
}

fn resolve_turn_id(payload: &JsonValue, current_turn_id: Option<&str>) -> Option<String> {
    payload
        .get("turn_id")
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| current_turn_id.map(ToOwned::to_owned))
}

fn tool_label(inner_type: Option<&str>, payload: &JsonValue) -> String {
    match inner_type {
        Some("function_call") | Some("custom_tool_call") => payload
            .get("name")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "tool_call".to_string()),
        Some("web_search_call") => "web_search".to_string(),
        Some("view_image_tool_call") => "view_image".to_string(),
        Some(other) => other.to_string(),
        None => "tool_call".to_string(),
    }
}

fn build_log_analytics(
    scope: &ResolvedDashboardScope,
    selected_project: Option<&str>,
) -> anyhow::Result<LogAnalytics> {
    let home_dir = dirs::home_dir().unwrap_or_default();
    let logs_path = home_dir.join(".codex").join("logs_2.sqlite");
    if !logs_path.exists() {
        return Ok(LogAnalytics::default());
    }

    let conn = Connection::open(logs_path)?;
    let (start_ts, end_ts) = scope_epoch_range(scope);
    let mut stmt = conn.prepare(
        "SELECT ts, level, target, COALESCE(feedback_log_body, '')
         FROM logs
         WHERE ts >= ?1 AND ts < ?2
         ORDER BY ts ASC",
    )?;
    let rows = stmt.query_map(params![start_ts, end_ts], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut analytics = LogAnalytics::default();
    for row in rows.filter_map(Result::ok) {
        let (ts, level, target, body) = row;
        let Some(timestamp) = unix_seconds_to_utc(ts) else {
            continue;
        };
        let local_date = to_local_date(timestamp);
        if !scope.contains_date(local_date) {
            continue;
        }
        let bucket = format_bucket_label(
            bucket_start(local_date, scope.granularity),
            scope.granularity,
        );
        let lower_target = target.to_ascii_lowercase();
        let lower_body = body.to_ascii_lowercase();
        if let Some(project) = selected_project {
            let log_cwd = extract_cwd_from_log(&body);
            if !matches_project_filter(log_cwd.as_deref(), Some(project)) {
                continue;
            }
        }

        if is_transport_target(&lower_target) && lower_body.contains("reconnect") {
            *analytics
                .signals
                .entry("Reconnect".to_string())
                .or_default() += 1;
            increment_breakdown(
                &mut analytics.transport_timeline,
                bucket.clone(),
                "Reconnect",
                1,
            );
        }
        if is_transport_target(&lower_target) && lower_body.contains("retry") {
            *analytics.signals.entry("Retry".to_string()).or_default() += 1;
            increment_breakdown(
                &mut analytics.transport_timeline,
                bucket.clone(),
                "Retry",
                1,
            );
        }
        if is_transport_error(&lower_target, &level, &lower_body) {
            *analytics
                .signals
                .entry("Transport error".to_string())
                .or_default() += 1;
            increment_breakdown(
                &mut analytics.transport_timeline,
                bucket.clone(),
                "Transport error",
                1,
            );
        }
        if let Some(tier) = parse_service_tier(&lower_body) {
            *analytics.speed_tiers.entry(tier.clone()).or_default() += 1;
            increment_breakdown(&mut analytics.speed_timeline, bucket, &tier, 1);
        }
    }

    Ok(analytics)
}

fn scope_epoch_range(scope: &ResolvedDashboardScope) -> (i64, i64) {
    let start = local_date_start_epoch(scope.start);
    let end = local_date_start_epoch(scope.end + Duration::days(1));
    (start, end)
}

fn local_date_start_epoch(date: NaiveDate) -> i64 {
    let naive = date.and_hms_opt(0, 0, 0).unwrap_or_default();
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .or_else(|| Local.from_local_datetime(&naive).latest())
        .map(|value| value.with_timezone(&Utc).timestamp())
        .unwrap_or_else(|| Utc.from_utc_datetime(&naive).timestamp())
}

fn is_transport_target(target: &str) -> bool {
    target.contains("transport")
        || target.contains("responses_websocket")
        || target.contains("stream_events_utils")
        || target.contains("codex_core::spawn")
}

fn is_transport_error(target: &str, level: &str, body: &str) -> bool {
    if !is_transport_target(target) {
        return false;
    }

    let level = level.to_ascii_lowercase();
    level == "error"
        || level == "warn"
        || body.contains("timeout")
        || body.contains("failed")
        || body.contains("error")
        || body.contains("disconnect")
}

fn parse_service_tier(body: &str) -> Option<String> {
    if !body.contains("service_tier") && !body.contains("service tier") {
        return None;
    }

    if body.contains("fast") {
        Some("Fast".to_string())
    } else if body.contains("standard") || body.contains("default") {
        Some("Standard".to_string())
    } else if body.contains("slow") {
        Some("Slow".to_string())
    } else {
        Some("Unknown".to_string())
    }
}

fn build_breakdown_series(
    timeline: &BTreeMap<String, HashMap<String, i64>>,
    categories: &[String],
) -> Vec<BreakdownDatum> {
    let mut rows = Vec::new();
    for (bucket, values) in timeline {
        for category in categories {
            rows.push(BreakdownDatum {
                bucket: bucket.clone(),
                category: category.clone(),
                value: *values.get(category).unwrap_or(&0),
            });
        }
    }
    rows
}

fn top_labels(map: &HashMap<String, i64>, limit: usize) -> Vec<String> {
    let mut items = map.iter().collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    items
        .into_iter()
        .take(limit)
        .map(|(label, _)| label.clone())
        .collect()
}

fn increment_breakdown(
    timeline: &mut BTreeMap<String, HashMap<String, i64>>,
    bucket: String,
    category: &str,
    amount: i64,
) {
    *timeline
        .entry(bucket)
        .or_default()
        .entry(category.to_string())
        .or_default() += amount;
}

fn bucket_label_for_timestamp(timestamp: DateTime<Utc>, granularity: TimeGranularity) -> String {
    format_bucket_label(
        bucket_start(to_local_date(timestamp), granularity),
        granularity,
    )
}

fn extract_search_queries(payload: &JsonValue) -> Vec<String> {
    let mut queries = Vec::new();
    if let Some(query) = payload
        .get("action")
        .and_then(|value| value.get("query"))
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        queries.push(query.trim().to_string());
    }
    if let Some(items) = payload
        .get("action")
        .and_then(|value| value.get("queries"))
        .and_then(JsonValue::as_array)
    {
        for item in items {
            if let Some(query) = item.as_str().filter(|value| !value.trim().is_empty()) {
                queries.push(query.trim().to_string());
            }
        }
    }
    queries
}

fn extract_last_token_usage(payload: &JsonValue) -> Option<TokenUsageAccumulator> {
    let usage = payload
        .get("info")
        .and_then(|value| value.get("last_token_usage"))?;
    Some(TokenUsageAccumulator {
        cached_input_tokens: usage
            .get("cached_input_tokens")
            .and_then(JsonValue::as_i64)
            .unwrap_or_default(),
        input_tokens: usage
            .get("input_tokens")
            .and_then(JsonValue::as_i64)
            .unwrap_or_default(),
        output_tokens: usage
            .get("output_tokens")
            .and_then(JsonValue::as_i64)
            .unwrap_or_default(),
        reasoning_output_tokens: usage
            .get("reasoning_output_tokens")
            .and_then(JsonValue::as_i64)
            .unwrap_or_default(),
        total_tokens: usage
            .get("total_tokens")
            .and_then(JsonValue::as_i64)
            .unwrap_or_default(),
    })
}

fn extract_cwd_from_log(body: &str) -> Option<String> {
    let marker = "cwd=";
    let start = body.find(marker)? + marker.len();
    let tail = &body[start..];
    let end = tail
        .find(|ch: char| matches!(ch, '}' | ',' | ')' | '"' | '\'' | '\n' | '\r'))
        .unwrap_or(tail.len());
    let value = tail[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn apply_tool_result(
    metric: &mut ToolMetricAccumulator,
    exit_code: Option<i64>,
    duration_sec: Option<f64>,
) {
    if exit_code.unwrap_or(0) == 0 {
        metric.success += 1;
    } else {
        metric.failure += 1;
    }
    if let Some(duration) = duration_sec.filter(|value| *value >= 0.0) {
        metric.duration_total_sec += duration;
        metric.duration_samples += 1;
    }
}

fn parse_function_call_exit_code(output: &str) -> Option<i64> {
    output
        .lines()
        .find_map(|line| line.strip_prefix("Exit code:"))
        .and_then(|value| value.trim().parse::<i64>().ok())
}

fn parse_wall_time_seconds(output: &str) -> Option<f64> {
    let value = output
        .lines()
        .find_map(|line| line.strip_prefix("Wall time:"))?
        .trim();
    let numeric = value
        .split_whitespace()
        .next()
        .and_then(|item| item.parse::<f64>().ok())?;
    if value.contains("millisecond") || value.contains("ms") {
        Some(numeric / 1000.0)
    } else {
        Some(numeric)
    }
}

fn parse_custom_tool_output(output: Option<&JsonValue>) -> (Option<i64>, Option<f64>) {
    let Some(raw) = output.and_then(JsonValue::as_str) else {
        return (None, None);
    };
    let Ok(parsed) = serde_json::from_str::<JsonValue>(raw) else {
        return (None, None);
    };
    let exit_code = parsed
        .get("metadata")
        .and_then(|value| value.get("exit_code"))
        .and_then(JsonValue::as_i64);
    let duration_sec = parsed
        .get("metadata")
        .and_then(|value| value.get("duration_seconds"))
        .and_then(JsonValue::as_f64)
        .or_else(|| {
            parsed
                .get("metadata")
                .and_then(|value| value.get("duration_seconds"))
                .and_then(JsonValue::as_i64)
                .map(|value| value as f64)
        });
    (exit_code, duration_sec)
}

fn contains_code_hint(text: &str) -> bool {
    text.contains("```") || text.matches('`').count() >= 2
}

fn contains_path_hint(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains(":\\")
        || lower.contains("\\")
        || lower.contains("~/")
        || lower.contains("../")
        || lower.contains("./")
        || (lower.contains('/') && !lower.contains("http://") && !lower.contains("https://"))
}

fn contains_command_hint(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "git ",
        "pnpm ",
        "npm ",
        "cargo ",
        "python ",
        "node ",
        "powershell ",
        "cmd ",
        "bash ",
        "uv ",
        "pytest ",
        "cd ",
        "ls ",
        "dir ",
        "rg ",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn normalize_workspace(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}

fn project_key(value: &str) -> String {
    last_path_component(value)
        .map(|item| item.to_ascii_lowercase())
        .unwrap_or_else(|| value.to_ascii_lowercase())
}

fn last_path_component(value: &str) -> Option<&str> {
    let trimmed = value
        .trim()
        .trim_end_matches(|character| matches!(character, '/' | '\\'));
    if trimmed.is_empty() {
        return None;
    }

    trimmed
        .rsplit(|character| matches!(character, '/' | '\\'))
        .find(|item| !item.trim().is_empty())
}

fn top_chart_data(map: &HashMap<String, i64>, limit: usize) -> Vec<ChartDatum> {
    top_chart_data_with_min_count(map, limit, 1)
}

fn top_chart_data_with_min_count(
    map: &HashMap<String, i64>,
    limit: usize,
    min_count: i64,
) -> Vec<ChartDatum> {
    let mut items = map
        .iter()
        .filter(|(_, value)| **value >= min_count)
        .map(|(label, value)| ChartDatum {
            label: label.clone(),
            value: *value,
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .value
            .cmp(&left.value)
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(limit);
    items
}

fn build_account_info() -> AccountInfo {
    let home_dir = dirs::home_dir().unwrap_or_default();
    let codex_root = home_dir.join(".codex");
    let auth_json = read_json_file(&codex_root.join("auth.json"));
    let global_state = read_json_file(&codex_root.join(".codex-global-state.json"));
    let config_text = fs::read_to_string(codex_root.join("config.toml")).ok();

    let last_refresh = auth_json
        .as_ref()
        .and_then(|value| value.get("last_refresh"))
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned);
    let auth_claims = auth_json
        .as_ref()
        .and_then(extract_auth_claims)
        .unwrap_or_default();

    AccountInfo {
        masked_email: auth_claims.email.as_deref().map(mask_email),
        plan_type: auth_claims.plan_type,
        masked_account_user_id: auth_claims.account_user_id.as_deref().map(mask_identifier),
        current_model: config_text
            .as_deref()
            .and_then(|text| extract_toml_value(text, "model")),
        current_reasoning_effort: config_text
            .as_deref()
            .and_then(|text| extract_toml_value(text, "model_reasoning_effort")),
        current_speed_tier: extract_default_service_tier(global_state.as_ref()),
        last_refresh,
    }
}

fn build_app_info(state: &AppState) -> AppInfo {
    let home_dir = dirs::home_dir().unwrap_or_default();
    let default_codex_root = home_dir.join(".codex");
    let session_index_path = default_codex_root.join("session_index.jsonl");

    AppInfo {
        default_codex_root: default_codex_root.to_string_lossy().into_owned(),
        database_path: state.db_path().to_string_lossy().into_owned(),
        session_index_path: session_index_path.to_string_lossy().into_owned(),
        platform: std::env::consts::OS.to_string(),
    }
}

fn update_prompt_terms(terms: &mut HashMap<String, i64>, text: &str) {
    let normalized = text.to_lowercase();
    let mut ascii_buffer = String::new();
    let mut cjk_buffer = String::new();

    for character in normalized.chars() {
        if is_ascii_term_char(character) {
            if !cjk_buffer.is_empty() {
                flush_cjk_buffer(&cjk_buffer, terms);
                cjk_buffer.clear();
            }
            ascii_buffer.push(character);
        } else if is_cjk(character) {
            if !ascii_buffer.is_empty() {
                flush_ascii_buffer(&ascii_buffer, terms);
                ascii_buffer.clear();
            }
            cjk_buffer.push(character);
        } else {
            if !ascii_buffer.is_empty() {
                flush_ascii_buffer(&ascii_buffer, terms);
                ascii_buffer.clear();
            }
            if !cjk_buffer.is_empty() {
                flush_cjk_buffer(&cjk_buffer, terms);
                cjk_buffer.clear();
            }
        }
    }

    if !ascii_buffer.is_empty() {
        flush_ascii_buffer(&ascii_buffer, terms);
    }
    if !cjk_buffer.is_empty() {
        flush_cjk_buffer(&cjk_buffer, terms);
    }
}

fn flush_ascii_buffer(buffer: &str, terms: &mut HashMap<String, i64>) {
    let token = buffer.trim_matches(|character: char| character == '-' || character == '_');
    if token.len() >= 2 {
        *terms.entry(token.to_string()).or_default() += 1;
    }
}

fn flush_cjk_buffer(buffer: &str, terms: &mut HashMap<String, i64>) {
    let chars = buffer.chars().collect::<Vec<_>>();
    if chars.len() < 2 {
        return;
    }

    if chars.len() <= 4 {
        *terms.entry(buffer.to_string()).or_default() += 1;
        return;
    }

    for window in chars.windows(2) {
        let token = window.iter().collect::<String>();
        *terms.entry(token).or_default() += 1;
    }
}

fn is_ascii_term_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '/' | ':' | '\\')
}

fn is_cjk(character: char) -> bool {
    matches!(
        character as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x20000..=0x2A6DF | 0x2A700..=0x2B73F
    )
}

fn read_json_file(path: &std::path::Path) -> Option<JsonValue> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::{last_path_component, normalize_workspace, project_key, project_label_from_cwd};

    #[test]
    fn extracts_project_names_from_windows_and_unix_paths() {
        assert_eq!(
            project_label_from_cwd(r"D:\Codes\rescue_codex"),
            "rescue_codex"
        );
        assert_eq!(
            project_label_from_cwd("/Users/tian/Codes/rescue_codex/"),
            "rescue_codex"
        );
        assert_eq!(
            last_path_component("~/Codes/rescue_codex"),
            Some("rescue_codex")
        );
    }

    #[test]
    fn normalizes_workspace_paths_independent_of_host_os() {
        assert_eq!(
            normalize_workspace(r"D:\Codes\rescue_codex"),
            "d:/codes/rescue_codex"
        );
        assert_eq!(project_key("d:/codes/rescue_codex"), "rescue_codex");
        assert_eq!(project_key(r"d:\codes\rescue_codex"), "rescue_codex");
    }
}

#[derive(Default)]
struct AuthClaims {
    email: Option<String>,
    plan_type: Option<String>,
    account_user_id: Option<String>,
}

fn extract_auth_claims(auth_json: &JsonValue) -> Option<AuthClaims> {
    let token = auth_json
        .get("tokens")
        .and_then(|value| value.get("access_token"))
        .and_then(JsonValue::as_str)?;
    let payload = decode_jwt_payload(token)?;
    let auth = payload
        .get("https://api.openai.com/auth")
        .cloned()
        .unwrap_or(JsonValue::Null);
    let profile = payload
        .get("https://api.openai.com/profile")
        .cloned()
        .unwrap_or(JsonValue::Null);

    Some(AuthClaims {
        email: profile
            .get("email")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
        plan_type: auth
            .get("chatgpt_plan_type")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
        account_user_id: auth
            .get("chatgpt_account_user_id")
            .or_else(|| auth.get("chatgpt_user_id"))
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
    })
}

fn decode_jwt_payload(token: &str) -> Option<JsonValue> {
    let payload = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&decoded).ok()
}

fn extract_default_service_tier(global_state: Option<&JsonValue>) -> Option<String> {
    global_state
        .and_then(|value| value.get("electron-persisted-atom-state"))
        .and_then(|value| value.get("default-service-tier"))
        .or_else(|| global_state.and_then(|value| value.get("default-service-tier")))
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
}

fn extract_toml_value(contents: &str, key: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }
        let Some((left, right)) = trimmed.split_once('=') else {
            continue;
        };
        if left.trim() == key {
            return Some(right.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return mask_identifier(email);
    };

    let visible_prefix = local.chars().take(2).collect::<String>();
    let visible_suffix = local
        .chars()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();

    format!("{visible_prefix}***{visible_suffix}@{domain}")
}

fn mask_identifier(value: &str) -> String {
    if value.len() <= 8 {
        return "***".to_string();
    }

    let prefix = value.chars().take(4).collect::<String>();
    let suffix = value
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();

    format!("{prefix}***{suffix}")
}

fn average_metric(values: impl Iterator<Item = f64>) -> f64 {
    let mut total = 0.0_f64;
    let mut count = 0.0_f64;
    for value in values {
        total += value;
        count += 1.0;
    }
    if count == 0.0 {
        0.0
    } else {
        total / count
    }
}

fn average_i64(total: i64, samples: i64) -> i64 {
    if samples <= 0 {
        0
    } else {
        (total as f64 / samples as f64).round() as i64
    }
}

fn average_f64(total: f64, samples: i64) -> f64 {
    if samples <= 0 {
        0.0
    } else {
        total / samples as f64
    }
}

fn parse_rfc3339_utc(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn parse_naive_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn unix_seconds_to_utc(value: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(value, 0).single()
}

fn to_local_date(value: DateTime<Utc>) -> NaiveDate {
    value.with_timezone(&Local).date_naive()
}

fn bucket_start(date: NaiveDate, granularity: TimeGranularity) -> NaiveDate {
    match granularity {
        TimeGranularity::Day => date,
        TimeGranularity::Week => {
            date - Duration::days(date.weekday().num_days_from_monday() as i64)
        }
        TimeGranularity::Month => {
            NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
        }
        TimeGranularity::Year => NaiveDate::from_ymd_opt(date.year(), 1, 1).unwrap_or(date),
    }
}

fn next_bucket_start(date: NaiveDate, granularity: TimeGranularity) -> NaiveDate {
    match granularity {
        TimeGranularity::Day => date + Duration::days(1),
        TimeGranularity::Week => date + Duration::days(7),
        TimeGranularity::Month => {
            let year = if date.month() == 12 {
                date.year() + 1
            } else {
                date.year()
            };
            let month = if date.month() == 12 {
                1
            } else {
                date.month() + 1
            };
            NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(date)
        }
        TimeGranularity::Year => NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap_or(date),
    }
}

fn format_bucket_label(date: NaiveDate, granularity: TimeGranularity) -> String {
    match granularity {
        TimeGranularity::Day => date.format("%Y-%m-%d").to_string(),
        TimeGranularity::Week => format!("Wk {}", date.format("%Y-%m-%d")),
        TimeGranularity::Month => date.format("%Y-%m").to_string(),
        TimeGranularity::Year => date.format("%Y").to_string(),
    }
}
