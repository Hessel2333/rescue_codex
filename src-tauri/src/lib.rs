mod commands;
mod db;
mod models;
mod parsers;
mod services;
mod state;

use state::AppState;
use std::fs;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let db_path = db::database_path(app.handle())?;
            db::init_database(&db_path)?;
            let media_dir = db_path
                .parent()
                .map(|path| path.join("media"))
                .unwrap_or_else(|| std::path::PathBuf::from("media"));
            fs::create_dir_all(&media_dir)?;
            app.manage(AppState::new(db_path, media_dir));
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
            commands::load_session_media,
            commands::export_report
        ])
        .run(tauri::generate_context!())
        .expect("failed to run rescue_codex");
}
