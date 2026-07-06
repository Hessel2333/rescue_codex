use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::UNIX_EPOCH,
};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    db::{now_iso, open_connection},
    models::{
        api::{ImportIssueRecord, ImportRunResult},
        parser::{ParseContext, ParseWarning, ParserTarget, SessionIndexEntry},
    },
    parsers::{message_hash, ParserRegistry},
    state::AppState,
};

pub fn start_scan_default_source(state: &AppState) -> anyhow::Result<ImportRunResult> {
    if !state.try_start_import() {
        anyhow::bail!("已有导入任务正在运行，请稍后再试");
    }

    let home_dir = dirs::home_dir().context("failed to resolve user home directory")?;
    let codex_root = home_dir.join(".codex");
    let worker_state = state.clone();

    thread::spawn(move || {
        let result = catch_unwind(AssertUnwindSafe(|| scan_default_source(&worker_state)));
        match result {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => eprintln!("background default import failed: {error:#}"),
            Err(_) => eprintln!("background default import panicked"),
        }
        worker_state.finish_import();
    });

    Ok(ImportRunResult {
        import_id: "background".to_string(),
        source_label: "Default Codex Home".to_string(),
        root_path: codex_root.to_string_lossy().into_owned(),
        status: "running".to_string(),
        files_total: 0,
        files_success: 0,
        files_failed: 0,
        warnings_count: 0,
        errors_count: 0,
        issues: Vec::new(),
    })
}

pub fn scan_default_source(state: &AppState) -> anyhow::Result<ImportRunResult> {
    let home_dir = dirs::home_dir().context("failed to resolve user home directory")?;
    let codex_root = home_dir.join(".codex");
    let sessions_root = codex_root.join("sessions");
    let archived_sessions_root = codex_root.join("archived_sessions");
    let session_index = load_session_index(&codex_root.join("session_index.jsonl"));
    let startup_warning = match (codex_root.exists(), sessions_root.exists()) {
        (false, _) => Some(ParseWarning {
            severity: "warning".to_string(),
            code: "codex_home_missing".to_string(),
            message: format!("默认 Codex 根目录不存在: {}", codex_root.display()),
            line_no: None,
            raw_excerpt: None,
        }),
        (true, false) => Some(ParseWarning {
            severity: "warning".to_string(),
            code: "sessions_root_missing".to_string(),
            message: format!("默认 sessions 目录不存在: {}", sessions_root.display()),
            line_no: None,
            raw_excerpt: None,
        }),
        _ => None,
    };
    let files = if sessions_root.exists() {
        let mut roots = vec![sessions_root];
        if archived_sessions_root.exists() {
            roots.push(archived_sessions_root);
        }
        collect_supported_files(&roots)
    } else {
        Vec::new()
    };

    let source_label = "Default Codex Home".to_string();
    run_import(
        state,
        files,
        "codex_home",
        &source_label,
        "scan_default",
        codex_root,
        Arc::new(session_index),
        startup_warning,
    )
}

pub fn start_import_paths(
    state: &AppState,
    raw_paths: Vec<String>,
) -> anyhow::Result<ImportRunResult> {
    if raw_paths.is_empty() {
        anyhow::bail!("no input paths provided");
    }
    if !state.try_start_import() {
        anyhow::bail!("已有导入任务正在运行，请稍后再试");
    }

    let paths = raw_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    let root = common_root(&paths).unwrap_or_else(|| PathBuf::from("."));
    let worker_state = state.clone();

    thread::spawn(move || {
        let result = catch_unwind(AssertUnwindSafe(|| import_paths(&worker_state, raw_paths)));
        match result {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => eprintln!("background manual import failed: {error:#}"),
            Err(_) => eprintln!("background manual import panicked"),
        }
        worker_state.finish_import();
    });

    Ok(ImportRunResult {
        import_id: "background".to_string(),
        source_label: "Manual Import".to_string(),
        root_path: root.to_string_lossy().into_owned(),
        status: "running".to_string(),
        files_total: 0,
        files_success: 0,
        files_failed: 0,
        warnings_count: 0,
        errors_count: 0,
        issues: Vec::new(),
    })
}

pub fn import_paths(state: &AppState, raw_paths: Vec<String>) -> anyhow::Result<ImportRunResult> {
    if raw_paths.is_empty() {
        anyhow::bail!("no input paths provided");
    }

    let paths = raw_paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    let files = collect_supported_files(&paths);
    let root = common_root(&paths).unwrap_or_else(|| PathBuf::from("."));
    let codex_root = detect_codex_root(&paths);
    let session_index = codex_root
        .as_ref()
        .map(|root| load_session_index(&root.join("session_index.jsonl")))
        .unwrap_or_default();

    run_import(
        state,
        files,
        "manual_import",
        "Manual Import",
        "manual_import",
        root,
        Arc::new(session_index),
        None,
    )
}

fn run_import(
    state: &AppState,
    files: Vec<PathBuf>,
    source_kind: &str,
    source_label: &str,
    mode: &str,
    root_path: PathBuf,
    session_index: Arc<HashMap<String, SessionIndexEntry>>,
    startup_warning: Option<ParseWarning>,
) -> anyhow::Result<ImportRunResult> {
    let mut conn = open_connection(state.db_path())?;
    let source_id = get_or_create_source(&conn, source_kind, source_label, &root_path)?;
    let import_id = Uuid::new_v4().to_string();
    let started_at = now_iso();

    conn.execute(
        "INSERT INTO imports (id, source_id, mode, parser_key, parser_version, status, started_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6)",
        params![import_id, source_id, mode, "auto", "registry-1", started_at],
    )?;
    conn.execute(
        "UPDATE imports SET files_total = ?2 WHERE id = ?1",
        params![import_id, files.len() as i64],
    )?;

    let mut files_success = 0_i64;
    let mut files_failed = 0_i64;
    let mut warnings_count = 0_i64;
    let mut errors_count = 0_i64;

    if let Some(warning) = startup_warning {
        warnings_count += 1;
        insert_issue(
            &conn,
            &import_id,
            None,
            &warning.severity,
            &warning.code,
            &warning.message,
            warning.line_no,
            warning.raw_excerpt.as_deref(),
        )?;
    }

    let registry = ParserRegistry::default();

    for path in &files {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                files_failed += 1;
                errors_count += 1;
                insert_issue(
                    &conn,
                    &import_id,
                    None,
                    "error",
                    "file_metadata_failed",
                    &format!("无法读取文件元信息: {} ({error})", path.display()),
                    None,
                    None,
                )?;
                update_import_progress(
                    &conn,
                    &import_id,
                    files_success,
                    files_failed,
                    warnings_count,
                    errors_count,
                )?;
                continue;
            }
        };

        let size_bytes = metadata.len() as i64;
        let mtime_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default();
        let rel_path = path.strip_prefix(&root_path).ok().map(Path::to_path_buf);
        let sha256 = hash_file(path)?;
        let source_file_id = ensure_source_file(
            &conn,
            &source_id,
            &import_id,
            path,
            rel_path.as_deref(),
            size_bytes,
            mtime_ms,
            &sha256,
            "processing",
        )?;

        if size_bytes == 0 {
            files_failed += 1;
            warnings_count += 1;
            update_source_file_status(&conn, &source_file_id, &import_id, "empty")?;
            insert_issue(
                &conn,
                &import_id,
                Some(&source_file_id),
                "warning",
                "empty_file",
                &format!("文件为空，已跳过: {}", path.display()),
                None,
                None,
            )?;
            update_import_progress(
                &conn,
                &import_id,
                files_success,
                files_failed,
                warnings_count,
                errors_count,
            )?;
            continue;
        }

        let target = ParserTarget {
            abs_path: path.clone(),
            rel_path: rel_path.clone(),
            extension: path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase(),
            sample: sample_file(path)?,
        };

        let parser = registry.resolve(&target)?;
        let ctx = ParseContext {
            source_id: source_id.clone(),
            import_id: import_id.clone(),
            abs_path: path.clone(),
            rel_path: rel_path.clone(),
            file_size: size_bytes as u64,
            mtime_ms,
            fingerprint: sha256,
            session_index: Arc::clone(&session_index),
        };

        match parser.parse(&target, &ctx) {
            Ok(result) => {
                if let Some(session) = &result.session {
                    persist_result(
                        &mut conn,
                        &import_id,
                        &source_file_id,
                        session.id.as_str(),
                        &result,
                    )?;
                    update_source_file_status(&conn, &source_file_id, &import_id, "imported")?;
                    files_success += 1;
                    warnings_count += result.warnings.len() as i64;
                } else {
                    files_failed += 1;
                    warnings_count += 1;
                    update_source_file_status(&conn, &source_file_id, &import_id, "no_session")?;
                    insert_issue(
                        &conn,
                        &import_id,
                        Some(&source_file_id),
                        "warning",
                        "missing_session",
                        &format!("解析结果未产生可用会话: {}", path.display()),
                        None,
                        None,
                    )?;
                }
            }
            Err(error) => {
                files_failed += 1;
                errors_count += 1;
                update_source_file_status(&conn, &source_file_id, &import_id, "failed")?;
                insert_issue(
                    &conn,
                    &import_id,
                    Some(&source_file_id),
                    "error",
                    "parse_failed",
                    &format!("解析失败: {} ({error})", path.display()),
                    None,
                    None,
                )?;
            }
        }

        update_import_progress(
            &conn,
            &import_id,
            files_success,
            files_failed,
            warnings_count,
            errors_count,
        )?;
    }

    let status = if errors_count > 0 {
        "completed_with_errors"
    } else if warnings_count > 0 {
        "completed_with_warnings"
    } else {
        "completed"
    };
    let finished_at = now_iso();

    conn.execute(
        "UPDATE imports
         SET status = ?2, finished_at = ?3, files_total = ?4, files_success = ?5, files_failed = ?6,
             warnings_count = ?7, errors_count = ?8
         WHERE id = ?1",
        params![
            import_id,
            status,
            finished_at,
            files.len() as i64,
            files_success,
            files_failed,
            warnings_count,
            errors_count
        ],
    )?;

    let issues = load_import_issues(&conn, &import_id)?;
    Ok(ImportRunResult {
        import_id,
        source_label: source_label.to_string(),
        root_path: root_path.to_string_lossy().into_owned(),
        status: status.to_string(),
        files_total: files.len() as i64,
        files_success,
        files_failed,
        warnings_count,
        errors_count,
        issues,
    })
}

fn persist_result(
    conn: &mut Connection,
    import_id: &str,
    source_file_id: &str,
    session_id: &str,
    result: &crate::models::parser::ParseResult,
) -> anyhow::Result<()> {
    let session = result
        .session
        .as_ref()
        .context("parse result missing session")?;
    let tx = conn.transaction()?;

    tx.execute(
        "DELETE FROM session_events_raw WHERE session_id = ?1",
        params![session_id],
    )?;
    tx.execute(
        "DELETE FROM session_messages WHERE session_id = ?1",
        params![session_id],
    )?;

    tx.execute(
        "INSERT INTO sessions (
            id, source_file_id, thread_title, cwd, originator, source, model_provider, cli_version,
            started_at, updated_at, first_user_message, raw_event_count, user_message_count,
            assistant_message_count, tool_call_count, turn_count, duration_sec, warning_count, warnings_json
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
         )
         ON CONFLICT(id) DO UPDATE SET
            source_file_id = excluded.source_file_id,
            thread_title = excluded.thread_title,
            cwd = excluded.cwd,
            originator = excluded.originator,
            source = excluded.source,
            model_provider = excluded.model_provider,
            cli_version = excluded.cli_version,
            started_at = excluded.started_at,
            updated_at = excluded.updated_at,
            first_user_message = excluded.first_user_message,
            raw_event_count = excluded.raw_event_count,
            user_message_count = excluded.user_message_count,
            assistant_message_count = excluded.assistant_message_count,
            tool_call_count = excluded.tool_call_count,
            turn_count = excluded.turn_count,
            duration_sec = excluded.duration_sec,
            warning_count = excluded.warning_count,
            warnings_json = excluded.warnings_json",
        params![
            session.id,
            source_file_id,
            session.thread_title,
            session.cwd,
            session.originator,
            session.source,
            session.model_provider,
            session.cli_version,
            session.started_at,
            session.updated_at,
            session.first_user_message,
            session.raw_event_count,
            session.user_message_count,
            session.assistant_message_count,
            session.tool_call_count,
            session.turn_count,
            session.duration_sec,
            session.warning_count,
            session.warnings_json,
        ],
    )?;

    for event in &result.events {
        tx.execute(
            "INSERT INTO session_events_raw (id, session_id, seq, ts, outer_type, inner_type, payload_json, warning_code)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Uuid::new_v4().to_string(),
                session.id,
                event.seq,
                event.ts,
                event.outer_type,
                event.inner_type,
                event.payload_json,
                event.warning_code
            ],
        )?;
    }

    for message in &result.messages {
        tx.execute(
            "INSERT INTO session_messages (id, session_id, turn_id, role, kind, text, ts, tool_name, phase, meta_json, text_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                Uuid::new_v4().to_string(),
                session.id,
                message.turn_id,
                message.role,
                message.kind,
                message.text,
                message.ts,
                message.tool_name,
                message.phase,
                message.meta_json,
                message_hash(message.text.as_deref().unwrap_or_default())
            ],
        )?;
    }

    for warning in &result.warnings {
        insert_issue_tx(
            &tx,
            import_id,
            Some(source_file_id),
            &warning.severity,
            &warning.code,
            &warning.message,
            warning.line_no,
            warning.raw_excerpt.as_deref(),
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn get_or_create_source(
    conn: &Connection,
    kind: &str,
    label: &str,
    root_path: &Path,
) -> anyhow::Result<String> {
    let root_text = root_path.to_string_lossy().into_owned();
    let existing = conn
        .query_row(
            "SELECT id FROM data_sources WHERE kind = ?1 AND root_path = ?2",
            params![kind, root_text],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    if let Some(id) = existing {
        conn.execute(
            "UPDATE data_sources SET label = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, label, now_iso()],
        )?;
        Ok(id)
    } else {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        conn.execute(
            "INSERT INTO data_sources (id, kind, label, root_path, config_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, '{}', ?5, ?6)",
            params![id, kind, label, root_text, now, now],
        )?;
        Ok(id)
    }
}

fn ensure_source_file(
    conn: &Connection,
    source_id: &str,
    import_id: &str,
    abs_path: &Path,
    rel_path: Option<&Path>,
    size_bytes: i64,
    mtime_ms: i64,
    sha256: &str,
    status: &str,
) -> anyhow::Result<String> {
    let abs_text = abs_path.to_string_lossy().into_owned();
    let existing = conn
        .query_row(
            "SELECT id FROM source_files WHERE abs_path = ?1 AND sha256 = ?2",
            params![abs_text, sha256],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    if let Some(id) = existing {
        conn.execute(
            "UPDATE source_files
             SET status = ?2, last_import_id = ?3, updated_at = ?4
             WHERE id = ?1",
            params![id, status, import_id, now_iso()],
        )?;
        return Ok(id);
    }

    let id = Uuid::new_v4().to_string();
    let now = now_iso();
    conn.execute(
        "INSERT INTO source_files (
            id, source_id, abs_path, rel_path, file_ext, size_bytes, mtime_ms, sha256, status,
            last_import_id, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id,
            source_id,
            abs_text,
            rel_path.map(|path| path.to_string_lossy().into_owned()),
            abs_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase(),
            size_bytes,
            mtime_ms,
            sha256,
            status,
            import_id,
            now,
            now
        ],
    )?;
    Ok(id)
}

fn update_source_file_status(
    conn: &Connection,
    source_file_id: &str,
    import_id: &str,
    status: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE source_files
         SET status = ?2, last_import_id = ?3, updated_at = ?4
         WHERE id = ?1",
        params![source_file_id, status, import_id, now_iso()],
    )?;
    Ok(())
}

fn update_import_progress(
    conn: &Connection,
    import_id: &str,
    files_success: i64,
    files_failed: i64,
    warnings_count: i64,
    errors_count: i64,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE imports
         SET files_success = ?2, files_failed = ?3, warnings_count = ?4, errors_count = ?5
         WHERE id = ?1",
        params![
            import_id,
            files_success,
            files_failed,
            warnings_count,
            errors_count
        ],
    )?;
    Ok(())
}

fn insert_issue(
    conn: &Connection,
    import_id: &str,
    source_file_id: Option<&str>,
    severity: &str,
    code: &str,
    message: &str,
    line_no: Option<i64>,
    raw_excerpt: Option<&str>,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO import_issues (id, import_id, source_file_id, severity, code, message, line_no, raw_excerpt, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            Uuid::new_v4().to_string(),
            import_id,
            source_file_id,
            severity,
            code,
            message,
            line_no,
            raw_excerpt,
            now_iso()
        ],
    )?;
    Ok(())
}

fn insert_issue_tx(
    tx: &Transaction<'_>,
    import_id: &str,
    source_file_id: Option<&str>,
    severity: &str,
    code: &str,
    message: &str,
    line_no: Option<i64>,
    raw_excerpt: Option<&str>,
) -> anyhow::Result<()> {
    tx.execute(
        "INSERT INTO import_issues (id, import_id, source_file_id, severity, code, message, line_no, raw_excerpt, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            Uuid::new_v4().to_string(),
            import_id,
            source_file_id,
            severity,
            code,
            message,
            line_no,
            raw_excerpt,
            now_iso()
        ],
    )?;
    Ok(())
}

fn load_import_issues(
    conn: &Connection,
    import_id: &str,
) -> anyhow::Result<Vec<ImportIssueRecord>> {
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
         WHERE import_id = ?1
         ORDER BY import_issues.created_at DESC
         LIMIT 20",
    )?;
    let rows = stmt.query_map(params![import_id], |row| {
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

fn collect_supported_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            if is_supported_file(path) {
                files.push(path.clone());
            }
            continue;
        }

        if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
                let candidate = entry.path();
                if candidate.is_file() && is_supported_file(candidate) {
                    files.push(candidate.to_path_buf());
                }
            }
        }
    }

    files.sort();
    files
}

fn is_supported_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "json" | "jsonl"
    )
}

fn common_root(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut components = paths.first()?.components().collect::<Vec<_>>();

    for path in &paths[1..] {
        let candidate = path.components().collect::<Vec<_>>();
        let shared = components
            .iter()
            .zip(candidate.iter())
            .take_while(|(left, right)| left == right)
            .count();
        components.truncate(shared);
    }

    if components.is_empty() {
        return None;
    }

    let mut root = PathBuf::new();
    for component in components {
        root.push(component.as_os_str());
    }
    Some(if root.is_file() {
        root.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        root
    })
}

fn detect_codex_root(paths: &[PathBuf]) -> Option<PathBuf> {
    for path in paths {
        for ancestor in path.ancestors() {
            if ancestor
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case(".codex"))
            {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn sample_file(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0_u8; 4096];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

fn hash_file(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let bytes = file.read(&mut buffer)?;
        if bytes == 0 {
            break;
        }
        hasher.update(&buffer[..bytes]);
    }

    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn load_session_index(path: &Path) -> HashMap<String, SessionIndexEntry> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return HashMap::new(),
    };
    let reader = BufReader::new(file);
    let mut map = HashMap::new();

    for line in reader.lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Some(id) = value.get("id").and_then(Value::as_str) {
            map.insert(
                id.to_string(),
                SessionIndexEntry {
                    thread_name: value
                        .get("thread_name")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    updated_at: value
                        .get("updated_at")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                },
            );
        }
    }

    map
}
