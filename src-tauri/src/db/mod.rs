use anyhow::Context;
use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

const INITIAL_MIGRATION: &str = include_str!("../../migrations/0001_init.sql");

pub fn database_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let app_dir = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data directory")?;
    fs::create_dir_all(&app_dir)?;
    Ok(app_dir.join("rescue_codex.sqlite"))
}

pub fn init_database(path: &Path) -> anyhow::Result<()> {
    let conn = open_connection(path)?;
    conn.execute_batch(INITIAL_MIGRATION)?;
    Ok(())
}

pub fn open_connection(path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        ",
    )?;
    Ok(conn)
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
