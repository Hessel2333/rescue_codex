mod commands;
mod db;
mod models;
mod parsers;
mod services;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db_path = db::database_path(app.handle())?;
            db::init_database(&db_path)?;
            app.manage(AppState::new(db_path));
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_default_source,
            commands::import_paths,
            commands::get_dashboard_summary,
            commands::list_sessions,
            commands::export_report
        ])
        .run(tauri::generate_context!())
        .expect("failed to run rescue_codex");
}
