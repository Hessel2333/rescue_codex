use std::fs;

use rusqlite::params;
use uuid::Uuid;

use crate::{
    db::{now_iso, open_connection},
    models::api::{ExportFormat, ExportRequest, ExportResult},
    state::AppState,
};

use super::query_service::{load_dashboard_summary, load_sessions};

pub fn export_report(state: &AppState, request: ExportRequest) -> anyhow::Result<ExportResult> {
    let conn = open_connection(state.db_path())?;
    let filters = request.filters.clone().unwrap_or_default();
    let content = match request.kind {
        crate::models::api::ExportKind::Dashboard => {
            let summary = load_dashboard_summary(
                &conn,
                state,
                request.dashboard_filters.clone().unwrap_or_default(),
                None,
            )?;
            match request.format {
                ExportFormat::Json => serde_json::to_string_pretty(&summary)?,
                ExportFormat::Markdown => dashboard_to_markdown(&summary),
                ExportFormat::Csv => dashboard_to_csv(&summary),
            }
        }
        crate::models::api::ExportKind::Sessions => {
            let response = load_sessions(&conn, &filters)?;
            match request.format {
                ExportFormat::Json => serde_json::to_string_pretty(&response)?,
                ExportFormat::Markdown => sessions_to_markdown(&response),
                ExportFormat::Csv => sessions_to_csv(&response)?,
            }
        }
    };

    fs::write(&request.path, content.as_bytes())?;

    let export_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO exports (id, format, scope_json, dest_path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            export_id,
            request.format.extension(),
            serde_json::to_string(&request)?,
            request.path,
            now_iso()
        ],
    )?;

    Ok(ExportResult {
        id: export_id,
        path: request.path,
        format: request.format,
        bytes_written: content.len() as u64,
    })
}

fn dashboard_to_markdown(summary: &crate::models::api::DashboardSummary) -> String {
    let mut output = String::new();
    output.push_str("# rescue_codex Dashboard\n\n");
    output.push_str("## Overview\n\n");
    output.push_str(&format!(
        "- Total sessions: {}\n",
        summary.overview.total_sessions
    ));
    output.push_str(&format!(
        "- Total questions: {}\n",
        summary.overview.total_questions
    ));
    output.push_str(&format!(
        "- Active days: {}\n",
        summary.overview.active_days
    ));
    output.push_str(&format!(
        "- Sessions last 7 days: {}\n",
        summary.overview.sessions_last_7_days
    ));
    output.push_str(&format!(
        "- Sessions last 30 days: {}\n",
        summary.overview.sessions_last_30_days
    ));
    output.push_str(&format!(
        "- Avg duration (sec): {:.2}\n",
        summary.overview.avg_duration_sec
    ));
    output.push_str(&format!(
        "- Avg turns: {:.2}\n",
        summary.overview.avg_turn_count
    ));
    output.push_str(&format!(
        "- Tool calls: {}\n\n",
        summary.overview.total_tool_calls
    ));
    output.push_str(&format!(
        "- Avg first response (sec): {:.2}\n",
        summary.overview.avg_first_response_sec
    ));
    output.push_str(&format!(
        "- Avg turn completion (sec): {:.2}\n\n",
        summary.overview.avg_turn_completion_sec
    ));

    output.push_str("## Recent Sessions\n\n");
    for session in &summary.recent_sessions {
        output.push_str(&format!(
            "- {} | {} | {} turns | {} tools\n",
            session
                .thread_title
                .clone()
                .unwrap_or_else(|| "Untitled".to_string()),
            session
                .cwd
                .clone()
                .unwrap_or_else(|| "Unknown cwd".to_string()),
            session.turn_count,
            session.tool_call_count
        ));
    }
    output
}

fn dashboard_to_csv(summary: &crate::models::api::DashboardSummary) -> String {
    let rows = vec![
        "metric,value".to_string(),
        format!("total_sessions,{}", summary.overview.total_sessions),
        format!("total_questions,{}", summary.overview.total_questions),
        format!("active_days,{}", summary.overview.active_days),
        format!(
            "sessions_last_7_days,{}",
            summary.overview.sessions_last_7_days
        ),
        format!(
            "sessions_last_30_days,{}",
            summary.overview.sessions_last_30_days
        ),
        format!("avg_duration_sec,{:.2}", summary.overview.avg_duration_sec),
        format!("avg_turn_count,{:.2}", summary.overview.avg_turn_count),
        format!("total_tool_calls,{}", summary.overview.total_tool_calls),
        format!(
            "avg_first_response_sec,{:.2}",
            summary.overview.avg_first_response_sec
        ),
        format!(
            "avg_turn_completion_sec,{:.2}",
            summary.overview.avg_turn_completion_sec
        ),
    ];
    rows.join("\n")
}

fn sessions_to_markdown(response: &crate::models::api::SessionListResponse) -> String {
    let mut output = String::new();
    output.push_str("# rescue_codex Sessions\n\n");
    output.push_str(&format!("Total: {}\n\n", response.total));
    for item in &response.items {
        output.push_str(&format!(
            "- {} | {} | {} turns | {} sec\n",
            item.thread_title
                .clone()
                .unwrap_or_else(|| "Untitled".to_string()),
            item.cwd
                .clone()
                .unwrap_or_else(|| "Unknown cwd".to_string()),
            item.turn_count,
            item.duration_sec
        ));
    }
    output
}

fn sessions_to_csv(response: &crate::models::api::SessionListResponse) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "id",
        "thread_title",
        "cwd",
        "source",
        "updated_at",
        "started_at",
        "duration_sec",
        "user_message_count",
        "assistant_message_count",
        "tool_call_count",
        "turn_count",
        "warning_count",
        "first_user_message",
    ])?;
    for item in &response.items {
        writer.write_record([
            item.id.as_str(),
            item.thread_title.as_deref().unwrap_or_default(),
            item.cwd.as_deref().unwrap_or_default(),
            item.source.as_deref().unwrap_or_default(),
            item.updated_at.as_deref().unwrap_or_default(),
            item.started_at.as_deref().unwrap_or_default(),
            &item.duration_sec.to_string(),
            &item.user_message_count.to_string(),
            &item.assistant_message_count.to_string(),
            &item.tool_call_count.to_string(),
            &item.turn_count.to_string(),
            &item.warning_count.to_string(),
            item.first_user_message.as_deref().unwrap_or_default(),
        ])?;
    }
    let bytes = writer.into_inner()?;
    Ok(String::from_utf8(bytes)?)
}
