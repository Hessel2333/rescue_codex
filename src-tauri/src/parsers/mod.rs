mod codex_session_jsonl;
mod generic_json;
mod generic_jsonl;

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use crate::models::parser::{
    ParseContext, ParseResult, ParseWarning, ParsedMessage, ParsedSession, ParserTarget,
};

pub use codex_session_jsonl::CodexSessionJsonlParser;
pub use generic_json::GenericJsonParser;
pub use generic_jsonl::GenericJsonlParser;

pub trait SourceParser: Send + Sync {
    fn key(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn supports(&self, target: &ParserTarget) -> u8;
    fn parse(&self, target: &ParserTarget, ctx: &ParseContext) -> anyhow::Result<ParseResult>;
}

pub struct ParserRegistry {
    parsers: Vec<Box<dyn SourceParser>>,
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self {
            parsers: vec![
                Box::new(CodexSessionJsonlParser),
                Box::new(GenericJsonlParser),
                Box::new(GenericJsonParser),
            ],
        }
    }
}

impl ParserRegistry {
    pub fn resolve<'a>(&'a self, target: &ParserTarget) -> anyhow::Result<&'a dyn SourceParser> {
        self.parsers
            .iter()
            .max_by_key(|parser| parser.supports(target))
            .map(|parser| parser.as_ref())
            .context("no parser available")
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionSeed {
    pub id: Option<String>,
    pub thread_title: Option<String>,
    pub cwd: Option<String>,
    pub originator: Option<String>,
    pub source: Option<String>,
    pub model_provider: Option<String>,
    pub cli_version: Option<String>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
}

pub fn finalize_session(
    seed: SessionSeed,
    ctx: &ParseContext,
    raw_event_count: usize,
    messages: &mut Vec<ParsedMessage>,
    warnings: &[ParseWarning],
) -> ParsedSession {
    dedupe_messages(messages);

    let user_message_count = messages
        .iter()
        .filter(|message| message.role.as_deref() == Some("user"))
        .count() as i64;
    let assistant_message_count = messages
        .iter()
        .filter(|message| message.role.as_deref() == Some("assistant"))
        .count() as i64;
    let tool_call_count = messages
        .iter()
        .filter(|message| message.kind == "tool_call")
        .count() as i64;

    let mut turn_ids = HashSet::new();
    for message in messages.iter() {
        if let Some(turn_id) = message.turn_id.clone() {
            if !turn_id.trim().is_empty() {
                turn_ids.insert(turn_id);
            }
        }
    }

    let turn_count = if turn_ids.is_empty() {
        user_message_count.max(assistant_message_count).max(1)
    } else {
        turn_ids.len() as i64
    };

    let first_user_message = messages
        .iter()
        .find(|message| message.role.as_deref() == Some("user"))
        .and_then(|message| message.text.clone());
    let first_user_ts = messages
        .iter()
        .find(|message| message.role.as_deref() == Some("user"))
        .and_then(|message| message.ts.clone());
    let first_any_ts = messages.iter().find_map(|message| message.ts.clone());
    let last_any_ts = messages.iter().rev().find_map(|message| message.ts.clone());

    let started_at = seed
        .started_at
        .clone()
        .or(first_user_ts)
        .or(first_any_ts)
        .or_else(|| iso_from_millis(ctx.mtime_ms));
    let updated_at = seed
        .updated_at
        .clone()
        .or(last_any_ts)
        .or_else(|| iso_from_millis(ctx.mtime_ms));

    let duration_sec = started_at
        .as_ref()
        .zip(updated_at.as_ref())
        .and_then(|(start, end)| diff_seconds(start, end))
        .unwrap_or_default();

    let warning_codes = warnings
        .iter()
        .map(|warning| warning.code.clone())
        .collect::<Vec<_>>();
    let warnings_json = serde_json::to_string(&warning_codes).unwrap_or_else(|_| "[]".to_string());

    ParsedSession {
        id: seed
            .id
            .unwrap_or_else(|| format!("synthetic-{}", short_hash(&ctx.fingerprint))),
        thread_title: seed.thread_title,
        cwd: seed.cwd,
        originator: seed.originator,
        source: seed.source,
        model_provider: seed.model_provider,
        cli_version: seed.cli_version,
        started_at,
        updated_at,
        first_user_message,
        raw_event_count: raw_event_count as i64,
        user_message_count,
        assistant_message_count,
        tool_call_count,
        turn_count,
        duration_sec,
        warning_count: warnings.len() as i64,
        warnings_json,
    }
}

pub fn dedupe_messages(messages: &mut Vec<ParsedMessage>) {
    let mut seen = HashMap::<String, Vec<Option<i64>>>::new();
    messages.retain(|message| {
        let key = format!(
            "{}|{}|{}|{}|{}|{}",
            message.role.clone().unwrap_or_default(),
            message.kind,
            message.turn_id.clone().unwrap_or_default(),
            message.tool_name.clone().unwrap_or_default(),
            message.phase.clone().unwrap_or_default(),
            message_hash(message.text.as_deref().unwrap_or(""))
        );
        let timestamp = message.ts.as_deref().and_then(timestamp_millis);
        let timestamps = seen.entry(key).or_default();
        let is_duplicate = timestamps
            .iter()
            .any(|existing| timestamps_overlap(*existing, timestamp));

        if is_duplicate {
            false
        } else {
            timestamps.push(timestamp);
            true
        }
    });
}

pub fn extract_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                if let Some(text) = item
                    .get("text")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("content").and_then(Value::as_str))
                {
                    if !text.trim().is_empty() {
                        parts.push(text.trim().to_string());
                    }
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n\n"))
            }
        }
        Value::Object(map) => map
            .get("text")
            .and_then(extract_text)
            .or_else(|| map.get("message").and_then(extract_text))
            .or_else(|| map.get("content").and_then(extract_text)),
        _ => None,
    }
}

pub fn generic_message_from_value(value: &Value, ts: Option<String>) -> Option<ParsedMessage> {
    let role = value
        .get("role")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            value
                .get("speaker")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        });
    let text = value
        .get("text")
        .and_then(extract_text)
        .or_else(|| value.get("message").and_then(extract_text))
        .or_else(|| value.get("content").and_then(extract_text));

    if role.is_none() && text.is_none() {
        return None;
    }

    Some(ParsedMessage {
        turn_id: value
            .get("turn_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        role,
        kind: "message".to_string(),
        text,
        ts,
        tool_name: None,
        phase: value
            .get("phase")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        meta_json: serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
    })
}

pub fn iso_from_millis(value: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(value).map(|dt| dt.to_rfc3339())
}

pub fn short_hash(seed: &str) -> String {
    seed.chars().take(16).collect()
}

pub fn message_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalize_text(text));
    format!("{:x}", hasher.finalize())
}

pub fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn diff_seconds(start: &str, end: &str) -> Option<i64> {
    let start = DateTime::parse_from_rfc3339(start).ok()?;
    let end = DateTime::parse_from_rfc3339(end).ok()?;
    Some((end - start).num_seconds().max(0))
}

pub fn timestamp_millis(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

fn timestamps_overlap(left: Option<i64>, right: Option<i64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => (left - right).abs() <= 2_000,
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::dedupe_messages;
    use crate::models::parser::ParsedMessage;

    fn message(text: &str, ts: Option<&str>) -> ParsedMessage {
        ParsedMessage {
            role: Some("assistant".to_string()),
            kind: "message".to_string(),
            text: Some(text.to_string()),
            ts: ts.map(ToOwned::to_owned),
            ..Default::default()
        }
    }

    #[test]
    fn dedupe_messages_collapses_nearby_duplicates() {
        let mut messages = vec![
            message("same payload", Some("2026-04-23T02:24:45.232Z")),
            message("same payload", Some("2026-04-23T02:24:45.233Z")),
        ];

        dedupe_messages(&mut messages);

        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn dedupe_messages_keeps_repeated_text_when_far_apart() {
        let mut messages = vec![
            message("same payload", Some("2026-04-23T02:24:45.232Z")),
            message("same payload", Some("2026-04-23T02:25:00.232Z")),
        ];

        dedupe_messages(&mut messages);

        assert_eq!(messages.len(), 2);
    }
}
