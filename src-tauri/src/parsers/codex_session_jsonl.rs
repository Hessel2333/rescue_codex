use crate::models::parser::{
    ParseContext, ParseResult, ParseWarning, ParsedMessage, ParsedRawEvent, ParserTarget,
    SessionIndexEntry,
};
use crate::parsers::{
    compact_json_string, extract_text, finalize_session, SessionSeed, SourceParser,
};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
};

pub struct CodexSessionJsonlParser;

impl SourceParser for CodexSessionJsonlParser {
    fn key(&self) -> &'static str {
        "codex_session_jsonl"
    }

    fn version(&self) -> &'static str {
        "2"
    }

    fn supports(&self, target: &ParserTarget) -> u8 {
        if target.extension != "jsonl" {
            return 0;
        }

        if target.sample.contains("\"type\": \"session_meta\"")
            || target.sample.contains("\"type\":\"session_meta\"")
        {
            100
        } else {
            10
        }
    }

    fn parse(&self, _target: &ParserTarget, ctx: &ParseContext) -> anyhow::Result<ParseResult> {
        let file = File::open(&ctx.abs_path)?;
        let reader = BufReader::new(file);

        let mut warnings = Vec::new();
        let mut events = Vec::new();
        let mut messages = Vec::new();
        let mut seen_turns = HashSet::new();
        let mut seed = SessionSeed::default();
        let mut last_ts = None;

        for (index, line) in reader.lines().enumerate() {
            let line_no = index as i64 + 1;
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let value: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(error) => {
                    warnings.push(ParseWarning {
                        severity: "warning".to_string(),
                        code: "invalid_json_line".to_string(),
                        message: format!("无法解析 JSONL 行: {error}"),
                        line_no: Some(line_no),
                        raw_excerpt: Some(line.chars().take(240).collect()),
                    });
                    continue;
                }
            };

            let timestamp = value
                .get("timestamp")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            if timestamp.is_some() {
                last_ts = timestamp.clone();
            }

            let outer_type = value
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let payload = value.get("payload").cloned().unwrap_or(Value::Null);
            let inner_type = payload
                .get("type")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);

            events.push(ParsedRawEvent {
                seq: events.len() as i64 + 1,
                ts: timestamp.clone(),
                outer_type: outer_type.clone(),
                inner_type: inner_type.clone(),
                payload_json: compact_codex_event_payload(&outer_type, &payload),
                warning_code: None,
            });

            if let Some(turn_id) = payload
                .get("turn_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
            {
                seen_turns.insert(turn_id);
            }

            if outer_type == "session_meta" {
                ingest_session_meta(&mut seed, &payload, &ctx.session_index);
            }

            if outer_type == "event_msg" && inner_type.as_deref() == Some("thread_name_updated") {
                seed.thread_title = payload
                    .get("thread_name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                if seed.updated_at.is_none() {
                    seed.updated_at = timestamp.clone();
                }
            }

            if let Some(message) = extract_codex_message(&outer_type, &payload, timestamp.clone()) {
                messages.push(message);
            }
        }

        if seed.updated_at.is_none() {
            seed.updated_at = last_ts;
        }

        if seed.id.is_none() {
            warnings.push(ParseWarning {
                severity: "warning".to_string(),
                code: "missing_session_id".to_string(),
                message: "未发现 session_meta.payload.id，已回退到 synthetic id".to_string(),
                line_no: None,
                raw_excerpt: None,
            });
        }

        let session = finalize_session(seed, ctx, events.len(), &mut messages, &warnings);
        let session = crate::models::parser::ParsedSession {
            turn_count: if seen_turns.is_empty() {
                session.turn_count
            } else {
                seen_turns.len() as i64
            },
            ..session
        };

        Ok(ParseResult {
            parser_key: self.key().to_string(),
            parser_version: self.version().to_string(),
            session: Some(session),
            events,
            messages,
            warnings,
            fingerprint: ctx.fingerprint.clone(),
        })
    }
}

fn ingest_session_meta(
    seed: &mut SessionSeed,
    payload: &Value,
    session_index: &std::sync::Arc<std::collections::HashMap<String, SessionIndexEntry>>,
) {
    seed.id = payload
        .get("id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    seed.cwd = payload
        .get("cwd")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    seed.originator = payload
        .get("originator")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    seed.source = payload
        .get("source")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    seed.model_provider = payload
        .get("model_provider")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    seed.cli_version = payload
        .get("cli_version")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    seed.started_at = payload
        .get("timestamp")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    if let Some(id) = seed.id.clone() {
        if let Some(entry) = session_index.get(&id) {
            if seed.thread_title.is_none() {
                seed.thread_title = entry.thread_name.clone();
            }
            if seed.updated_at.is_none() {
                seed.updated_at = entry.updated_at.clone();
            }
        }
    }
}

fn compact_codex_event_payload(outer_type: &str, payload: &Value) -> String {
    let Some(payload_type) = payload.get("type").and_then(Value::as_str) else {
        return compact_json_string(payload);
    };

    let mut compact = serde_json::Map::new();
    insert_string(&mut compact, "type", Some(payload_type));
    insert_string(
        &mut compact,
        "turn_id",
        payload.get("turn_id").and_then(Value::as_str),
    );

    match (outer_type, payload_type) {
        ("event_msg", "task_started") => {
            insert_i64(
                &mut compact,
                "started_at",
                payload.get("started_at").and_then(Value::as_i64),
            );
        }
        ("turn_context", _) => {
            insert_string(
                &mut compact,
                "model",
                payload.get("model").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "effort",
                payload.get("effort").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "cwd",
                payload.get("cwd").and_then(Value::as_str),
            );
            if let Some(mode) = payload.get("collaboration_mode") {
                compact.insert("collaboration_mode".to_string(), mode.clone());
            }
        }
        ("event_msg", "user_message") => {
            insert_string(
                &mut compact,
                "message",
                payload.get("message").and_then(Value::as_str),
            );
        }
        ("event_msg", "task_complete") => {
            insert_i64(
                &mut compact,
                "completed_at",
                payload.get("completed_at").and_then(Value::as_i64),
            );
            insert_i64(
                &mut compact,
                "duration_ms",
                payload.get("duration_ms").and_then(Value::as_i64),
            );
        }
        ("event_msg", "agent_reasoning")
        | ("event_msg", "agent_message")
        | ("event_msg", "turn_aborted")
        | ("event_msg", "thread_rolled_back")
        | ("event_msg", "context_compacted")
        | ("compacted", _) => {}
        ("event_msg", "token_count") | ("token_count", _) => {
            if let Some(info) = payload.get("info") {
                compact.insert("info".to_string(), info.clone());
            }
        }
        ("response_item", "function_call")
        | ("response_item", "custom_tool_call")
        | ("response_item", "web_search_call") => {
            insert_string(
                &mut compact,
                "name",
                payload.get("name").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "call_id",
                payload.get("call_id").and_then(Value::as_str),
            );
            if let Some(action) = payload.get("action") {
                compact.insert("action".to_string(), action.clone());
            }
        }
        ("response_item", "function_call_output")
        | ("response_item", "custom_tool_call_output") => {
            insert_string(
                &mut compact,
                "call_id",
                payload.get("call_id").and_then(Value::as_str),
            );
            if let Some(output) = payload.get("output") {
                compact.insert("output".to_string(), output.clone());
            }
        }
        _ => {}
    }

    compact_json_string(&Value::Object(compact))
}

fn insert_string(map: &mut serde_json::Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn insert_i64(map: &mut serde_json::Map<String, Value>, key: &str, value: Option<i64>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::Number(value.into()));
    }
}

fn extract_codex_message(
    outer_type: &str,
    payload: &Value,
    timestamp: Option<String>,
) -> Option<ParsedMessage> {
    match outer_type {
        "event_msg" => match payload.get("type").and_then(Value::as_str) {
            Some("user_message") => Some(ParsedMessage {
                turn_id: payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                role: Some("user".to_string()),
                kind: "message".to_string(),
                text: payload.get("message").and_then(extract_text),
                ts: timestamp,
                tool_name: None,
                phase: None,
                meta_json: compact_json_string(payload),
            }),
            Some("agent_message") => Some(ParsedMessage {
                turn_id: payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                role: Some("assistant".to_string()),
                kind: "message".to_string(),
                text: payload.get("message").and_then(extract_text),
                ts: timestamp,
                tool_name: None,
                phase: payload
                    .get("phase")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                meta_json: compact_json_string(payload),
            }),
            _ => None,
        },
        "response_item" => match payload.get("type").and_then(Value::as_str) {
            Some("message") => extract_response_message(payload, timestamp),
            Some("function_call") | Some("custom_tool_call") => Some(ParsedMessage {
                turn_id: payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                role: Some("assistant".to_string()),
                kind: "tool_call".to_string(),
                text: payload
                    .get("arguments")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                ts: timestamp,
                tool_name: payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                phase: None,
                meta_json: compact_json_string(payload),
            }),
            Some("function_call_output") | Some("custom_tool_call_output") => Some(ParsedMessage {
                turn_id: payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                role: Some("tool".to_string()),
                kind: "tool_result".to_string(),
                text: payload
                    .get("output")
                    .and_then(extract_text)
                    .or_else(|| payload.get("content").and_then(extract_text)),
                ts: timestamp,
                tool_name: payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                phase: None,
                meta_json: compact_json_string(payload),
            }),
            _ => None,
        },
        _ => None,
    }
}

fn extract_response_message(payload: &Value, timestamp: Option<String>) -> Option<ParsedMessage> {
    let role = payload.get("role").and_then(Value::as_str)?;
    if matches!(role, "developer" | "system") {
        return None;
    }

    let text = payload.get("content").and_then(extract_text);
    if text.as_deref().is_some_and(is_internal_scaffold_message) {
        return None;
    }

    Some(ParsedMessage {
        turn_id: payload
            .get("turn_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        role: Some(role.to_string()),
        kind: "message".to_string(),
        text,
        ts: timestamp,
        tool_name: None,
        phase: payload
            .get("phase")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        meta_json: compact_json_string(payload),
    })
}

fn is_internal_scaffold_message(text: &str) -> bool {
    let trimmed = text.trim_start();
    [
        "<environment_context>",
        "<environment_context",
        "<permissions instructions>",
        "<permissions instructions",
        "<app-context>",
        "<app-context",
        "<collaboration_mode>",
        "<collaboration_mode",
        "<skills_instructions>",
        "<skills_instructions",
        "<plugins_instructions>",
        "<plugins_instructions",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::{extract_codex_message, is_internal_scaffold_message};
    use serde_json::json;

    #[test]
    fn filters_internal_response_messages() {
        let developer_payload = json!({
            "type": "message",
            "role": "developer",
            "content": [{"type": "input_text", "text": "<permissions instructions>secret</permissions instructions>"}]
        });
        assert!(extract_codex_message("response_item", &developer_payload, None).is_none());

        let user_payload = json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "<environment_context>\n  <cwd>D:\\\\Codes\\\\rescue_codex</cwd>\n</environment_context>"}]
        });
        assert!(extract_codex_message("response_item", &user_payload, None).is_none());
        assert!(is_internal_scaffold_message(
            "<environment_context>\n  <cwd>D:\\Codes\\rescue_codex</cwd>\n</environment_context>"
        ));
    }

    #[test]
    fn keeps_visible_assistant_response_messages() {
        let payload = json!({
            "type": "message",
            "role": "assistant",
            "phase": "final_answer",
            "content": [{"type": "output_text", "text": "<proposed_plan>\n# rescue_codex 第一阶段方案"}]
        });

        let message = extract_codex_message("response_item", &payload, None)
            .expect("assistant response_item should be kept");

        assert_eq!(message.role.as_deref(), Some("assistant"));
        assert_eq!(message.phase.as_deref(), Some("final_answer"));
        assert_eq!(
            message.text.as_deref(),
            Some("<proposed_plan>\n# rescue_codex 第一阶段方案")
        );
    }
}
