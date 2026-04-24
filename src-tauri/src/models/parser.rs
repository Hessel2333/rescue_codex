use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionIndexEntry {
    pub thread_name: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParseContext {
    pub source_id: String,
    pub import_id: String,
    pub abs_path: PathBuf,
    pub rel_path: Option<PathBuf>,
    pub file_size: u64,
    pub mtime_ms: i64,
    pub fingerprint: String,
    pub session_index: Arc<HashMap<String, SessionIndexEntry>>,
}

#[derive(Debug, Clone)]
pub struct ParserTarget {
    pub abs_path: PathBuf,
    pub rel_path: Option<PathBuf>,
    pub extension: String,
    pub sample: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParseWarning {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub line_no: Option<i64>,
    pub raw_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSession {
    pub id: String,
    pub thread_title: Option<String>,
    pub cwd: Option<String>,
    pub originator: Option<String>,
    pub source: Option<String>,
    pub model_provider: Option<String>,
    pub cli_version: Option<String>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub first_user_message: Option<String>,
    pub raw_event_count: i64,
    pub user_message_count: i64,
    pub assistant_message_count: i64,
    pub tool_call_count: i64,
    pub turn_count: i64,
    pub duration_sec: i64,
    pub warning_count: i64,
    pub warnings_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRawEvent {
    pub seq: i64,
    pub ts: Option<String>,
    pub outer_type: String,
    pub inner_type: Option<String>,
    pub payload_json: String,
    pub warning_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParsedMessage {
    pub turn_id: Option<String>,
    pub role: Option<String>,
    pub kind: String,
    pub text: Option<String>,
    pub ts: Option<String>,
    pub tool_name: Option<String>,
    pub phase: Option<String>,
    pub meta_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub parser_key: String,
    pub parser_version: String,
    pub session: Option<ParsedSession>,
    pub events: Vec<ParsedRawEvent>,
    pub messages: Vec<ParsedMessage>,
    pub warnings: Vec<ParseWarning>,
    pub fingerprint: String,
}
