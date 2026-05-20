use tauri::State;

use crate::{
    models::api::{
        DashboardFilters, DashboardSummary, ExportRequest, ExportResult, ImportRunResult,
        SessionListFilters, SessionListResponse,
    },
    services::{export_service, import_service, query_service},
    state::AppState,
};

#[tauri::command]
pub fn scan_default_source(state: State<'_, AppState>) -> Result<ImportRunResult, String> {
    import_service::start_scan_default_source(&state).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ImportRunResult, String> {
    import_service::start_import_paths(&state, paths).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_dashboard_summary(
    state: State<'_, AppState>,
    filters: Option<DashboardFilters>,
    section: Option<String>,
) -> Result<DashboardSummary, String> {
    query_service::get_dashboard_summary(&state, filters.unwrap_or_default(), section)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_sessions(
    state: State<'_, AppState>,
    filters: SessionListFilters,
) -> Result<SessionListResponse, String> {
    query_service::list_sessions(&state, filters).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_report(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResult, String> {
    export_service::export_report(&state, request).map_err(|error| error.to_string())
}
