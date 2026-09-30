use crate::models::parser::{
    ParseContext, ParseResult, ParseWarning, ParsedMedia, ParsedMessage, ParsedRawEvent,
    ParserTarget, SessionIndexEntry,
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
        "6"
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
    if outer_type == "session_meta" {
        return compact_json_string(&compact_session_meta(payload));
    }

    if outer_type == "turn_context" {
        return compact_json_string(&compact_turn_context(payload));
    }

    if outer_type == "token_usage_record" {
        return compact_json_string(&compact_token_usage_record(payload));
    }

    if outer_type == "world_state" {
        return compact_json_string(&compact_world_state(payload));
    }

    if outer_type == "compacted" {
        let mut compact = serde_json::Map::new();
        compact.insert("type".to_string(), Value::String("compacted".to_string()));
        insert_string(
            &mut compact,
            "turn_id",
            payload.get("turn_id").and_then(Value::as_str),
        );
        return compact_json_string(&Value::Object(compact));
    }

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
        ("event_msg", "user_message") => {
            insert_string(
                &mut compact,
                "message",
                payload.get("message").and_then(Value::as_str),
            );
        }
        ("response_item", "message") => {
            let role = payload.get("role").and_then(Value::as_str);
            if matches!(role, Some("user" | "assistant")) {
                let text = visible_message_text(payload);
                if !payload.get("content").and_then(extract_text)
                    .as_deref().is_some_and(is_internal_scaffold_message)
                {
                    insert_string(&mut compact, "role", role);
                    insert_string(&mut compact, "text", text.as_deref());
                }
            }
        }
        ("event_msg", "task_complete") => {
            insert_i64(
                &mut compact,
                "time_to_first_token_ms",
                payload.get("time_to_first_token_ms").and_then(Value::as_i64),
            );
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
        ("event_msg", "thread_settings_applied") => {
            if let Some(settings) = payload.get("thread_settings") {
                let mut compact_settings = serde_json::Map::new();
                for key in ["model", "reasoning_effort", "cwd", "service_tier"] {
                    insert_string(
                        &mut compact_settings,
                        key,
                        settings.get(key).and_then(Value::as_str),
                    );
                }
                if let Some(mode) = settings.get("collaboration_mode") {
                    compact_settings.insert(
                        "collaboration_mode".to_string(),
                        compact_collaboration_mode(mode),
                    );
                }
                compact.insert(
                    "thread_settings".to_string(),
                    Value::Object(compact_settings),
                );
            }
        }
        ("event_msg", "agent_reasoning")
        | ("event_msg", "agent_message")
        | ("event_msg", "turn_aborted")
        | ("event_msg", "thread_rolled_back")
        | ("event_msg", "context_compacted")
        | ("compacted", _) => {}
        ("event_msg", "mcp_tool_call_end") => {
            insert_string(
                &mut compact,
                "call_id",
                payload.get("call_id").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "server",
                mcp_invocation_string(payload, "server"),
            );
            insert_string(&mut compact, "tool", mcp_invocation_string(payload, "tool"));
            insert_string(&mut compact, "title", mcp_invocation_title(payload));
            insert_f64(&mut compact, "duration_sec", duration_seconds(payload));
            insert_bool(&mut compact, "is_error", result_is_error(payload));
            insert_string(
                &mut compact,
                "result_text",
                mcp_result_text(payload).as_deref(),
            );
        }
        ("event_msg", "image_generation_end") => {
            insert_string(
                &mut compact,
                "call_id",
                payload.get("call_id").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "status",
                payload.get("status").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "saved_path",
                payload.get("saved_path").and_then(Value::as_str),
            );
        }
        ("event_msg", "patch_apply_end") => {
            insert_string(
                &mut compact,
                "call_id",
                payload.get("call_id").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "status",
                payload.get("status").and_then(Value::as_str),
            );
            insert_bool(
                &mut compact,
                "success",
                payload.get("success").and_then(Value::as_bool),
            );
            insert_f64(&mut compact, "duration_sec", duration_seconds(payload));
        }
        ("event_msg", "web_search_end") => {
            insert_string(
                &mut compact,
                "call_id",
                payload.get("call_id").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "query",
                payload.get("query").and_then(Value::as_str),
            );
            if let Some(action) = payload.get("action") {
                compact.insert("action".to_string(), compact_search_action(action));
            }
        }
        ("event_msg", "sub_agent_activity") => {
            insert_string(
                &mut compact,
                "event_id",
                payload.get("event_id").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "agent_thread_id",
                payload.get("agent_thread_id").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "agent_path",
                payload.get("agent_path").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "kind",
                payload.get("kind").and_then(Value::as_str),
            );
            insert_i64(
                &mut compact,
                "occurred_at_ms",
                payload.get("occurred_at_ms").and_then(Value::as_i64),
            );
        }
        ("event_msg", "item_completed") => {
            insert_i64(
                &mut compact,
                "started_at_ms",
                payload.get("started_at_ms").and_then(Value::as_i64),
            );
            insert_i64(
                &mut compact,
                "completed_at_ms",
                payload.get("completed_at_ms").and_then(Value::as_i64),
            );
            if let Some(item) = payload.get("item") {
                compact.insert("item".to_string(), compact_completed_item(item));
            }
        }
        ("event_msg", "token_count") | ("token_count", _) => {
            if let Some(info) = payload.get("info") {
                compact.insert("info".to_string(), info.clone());
            }
        }
        ("response_item", "function_call")
        | ("response_item", "custom_tool_call")
        | ("response_item", "web_search_call")
        | ("response_item", "tool_search_call")
        | ("response_item", "image_generation_call") => {
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
            insert_string(
                &mut compact,
                "id",
                payload.get("id").and_then(Value::as_str),
            );
            insert_string(
                &mut compact,
                "status",
                payload.get("status").and_then(Value::as_str),
            );
            if let Some(action) = payload.get("action") {
                compact.insert("action".to_string(), compact_search_action(action));
            }
        }
        ("response_item", "function_call_output")
        | ("response_item", "custom_tool_call_output")
        | ("response_item", "tool_search_output") => {
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

fn compact_session_meta(payload: &Value) -> Value {
    let mut compact = serde_json::Map::new();
    compact.insert(
        "type".to_string(),
        Value::String("session_meta".to_string()),
    );
    for key in [
        "id",
        "cwd",
        "originator",
        "source",
        "model_provider",
        "cli_version",
        "timestamp",
    ] {
        insert_string(&mut compact, key, payload.get(key).and_then(Value::as_str));
    }
    Value::Object(compact)
}

fn compact_turn_context(payload: &Value) -> Value {
    let mut compact = serde_json::Map::new();
    compact.insert(
        "type".to_string(),
        Value::String("turn_context".to_string()),
    );
    for key in ["turn_id", "model", "effort", "cwd"] {
        insert_string(&mut compact, key, payload.get(key).and_then(Value::as_str));
    }
    if let Some(mode) = payload.get("collaboration_mode") {
        compact.insert(
            "collaboration_mode".to_string(),
            compact_collaboration_mode(mode),
        );
    }
    Value::Object(compact)
}

fn compact_collaboration_mode(mode: &Value) -> Value {
    let Some(mode_object) = mode.as_object() else {
        return mode.clone();
    };
    let mut compact = serde_json::Map::new();
    for key in ["mode", "name"] {
        insert_string(
            &mut compact,
            key,
            mode_object.get(key).and_then(Value::as_str),
        );
    }
    if let Some(settings) = mode_object.get("settings") {
        let mut compact_settings = serde_json::Map::new();
        insert_string(
            &mut compact_settings,
            "reasoning_effort",
            settings.get("reasoning_effort").and_then(Value::as_str),
        );
        if !compact_settings.is_empty() {
            compact.insert("settings".to_string(), Value::Object(compact_settings));
        }
    }
    Value::Object(compact)
}

fn compact_token_usage_record(payload: &Value) -> Value {
    let mut compact = serde_json::Map::new();
    compact.insert(
        "type".to_string(),
        Value::String("token_usage_record".to_string()),
    );
    for key in ["turn_id", "response_id"] {
        insert_string(&mut compact, key, payload.get(key).and_then(Value::as_str));
    }
    if let Some(usage) = payload.get("usage") {
        compact.insert("usage".to_string(), compact_token_usage(usage));
    }
    Value::Object(compact)
}

fn compact_token_usage(usage: &Value) -> Value {
    let mut compact = serde_json::Map::new();
    for key in [
        "input_tokens",
        "cached_input_tokens",
        "cache_write_input_tokens",
        "output_tokens",
        "reasoning_output_tokens",
        "total_tokens",
    ] {
        insert_i64(&mut compact, key, usage.get(key).and_then(Value::as_i64));
    }
    Value::Object(compact)
}

fn compact_world_state(payload: &Value) -> Value {
    let mut compact = serde_json::Map::new();
    compact.insert("type".to_string(), Value::String("world_state".to_string()));
    insert_bool(
        &mut compact,
        "full",
        payload.get("full").and_then(Value::as_bool),
    );
    if let Some(state) = payload.get("state") {
        insert_string(
            &mut compact,
            "model",
            state.get("model").and_then(Value::as_str),
        );
        insert_string(
            &mut compact,
            "cwd",
            state.get("cwd").and_then(Value::as_str),
        );
    }
    Value::Object(compact)
}

fn compact_search_action(action: &Value) -> Value {
    let mut compact = serde_json::Map::new();
    insert_string(
        &mut compact,
        "type",
        action.get("type").and_then(Value::as_str),
    );
    insert_string(
        &mut compact,
        "query",
        action.get("query").and_then(Value::as_str),
    );
    if let Some(queries) = action.get("queries") {
        compact.insert("queries".to_string(), queries.clone());
    }
    Value::Object(compact)
}

fn compact_completed_item(item: &Value) -> Value {
    let mut compact = serde_json::Map::new();
    for key in [
        "type",
        "id",
        "status",
        "kind",
        "command",
        "cwd",
        "path",
        "server",
        "tool",
        "appName",
        "pluginId",
        "actionName",
        "query",
    ] {
        insert_string(&mut compact, key, item.get(key).and_then(Value::as_str));
    }
    insert_i64(
        &mut compact,
        "exit_code",
        item.get("exit_code").and_then(Value::as_i64),
    );
    insert_bool(
        &mut compact,
        "readOnlyHint",
        item.get("readOnlyHint").and_then(Value::as_bool),
    );
    if matches!(item.get("type").and_then(Value::as_str), Some("UserMessage" | "AgentMessage")) {
        let text = visible_message_text(item);
        insert_string(&mut compact, "text", text.as_deref());
    }
    if let Some(duration) = item.get("duration") {
        compact.insert("duration".to_string(), duration.clone());
    }
    if let Some(arguments) = item.get("arguments") {
        compact.insert("arguments".to_string(), arguments.clone());
    }
    if let Some(action) = item.get("action") {
        compact.insert("action".to_string(), compact_search_action(action));
    }
    Value::Object(compact)
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

fn insert_f64(map: &mut serde_json::Map<String, Value>, key: &str, value: Option<f64>) {
    if let Some(value) = value.and_then(serde_json::Number::from_f64) {
        map.insert(key.to_string(), Value::Number(value));
    }
}

fn insert_bool(map: &mut serde_json::Map<String, Value>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::Bool(value));
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
                media: extract_image_media(payload),
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
                media: extract_image_media(payload),
            }),
            Some("mcp_tool_call_end") => Some(ParsedMessage {
                turn_id: payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                role: Some("assistant".to_string()),
                kind: "tool_call".to_string(),
                text: mcp_message_text(payload),
                ts: timestamp,
                tool_name: mcp_tool_name(payload),
                phase: None,
                meta_json: compact_codex_event_payload(outer_type, payload),
                media: extract_image_media(payload),
            }),
            Some("image_generation_end") => Some(ParsedMessage {
                turn_id: payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                role: Some("tool".to_string()),
                kind: "tool_result".to_string(),
                text: image_generation_message_text(payload),
                ts: timestamp,
                tool_name: Some("image_generation".to_string()),
                phase: None,
                meta_json: compact_codex_event_payload(outer_type, payload),
                media: extract_image_generation_media(payload),
            }),
            Some("sub_agent_activity") => Some(ParsedMessage {
                turn_id: payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                role: Some("assistant".to_string()),
                kind: "tool_call".to_string(),
                text: sub_agent_message_text(payload),
                ts: timestamp,
                tool_name: Some("sub_agent".to_string()),
                phase: None,
                meta_json: compact_codex_event_payload(outer_type, payload),
                media: Vec::new(),
            }),
            _ => None,
        },
        "response_item" => match payload.get("type").and_then(Value::as_str) {
            Some("message") => extract_response_message(payload, timestamp),
            Some("function_call")
            | Some("custom_tool_call")
            | Some("tool_search_call")
            | Some("image_generation_call") => Some(ParsedMessage {
                turn_id: payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                role: Some("assistant".to_string()),
                kind: "tool_call".to_string(),
                text: payload
                    .get("arguments")
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("input").and_then(Value::as_str))
                    .map(ToOwned::to_owned),
                ts: timestamp,
                tool_name: tool_name_from_response_item(payload),
                phase: None,
                meta_json: compact_json_string(payload),
                media: extract_image_media(payload),
            }),
            Some("function_call_output")
            | Some("custom_tool_call_output")
            | Some("tool_search_output") => Some(ParsedMessage {
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
                tool_name: tool_name_from_response_item(payload),
                phase: None,
                meta_json: compact_json_string(payload),
                media: extract_image_media(payload),
            }),
            _ => None,
        },
        _ => None,
    }
}

fn tool_name_from_response_item(payload: &Value) -> Option<String> {
    payload
        .get("name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            payload.get("type").and_then(Value::as_str).map(|value| {
                value
                    .trim_end_matches("_call")
                    .trim_end_matches("_output")
                    .to_string()
            })
        })
}

fn mcp_invocation_string<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload
        .get("invocation")
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .or_else(|| payload.get(key).and_then(Value::as_str))
}

fn mcp_invocation_title(payload: &Value) -> Option<&str> {
    payload
        .get("invocation")
        .and_then(|value| value.get("arguments"))
        .and_then(|value| value.get("title"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("title").and_then(Value::as_str))
}

fn mcp_tool_name(payload: &Value) -> Option<String> {
    let tool = mcp_invocation_string(payload, "tool")?;
    Some(match mcp_invocation_string(payload, "server") {
        Some(server) if !server.trim().is_empty() => format!("{server}.{tool}"),
        _ => tool.to_string(),
    })
}

fn mcp_message_text(payload: &Value) -> Option<String> {
    mcp_invocation_title(payload)
        .map(ToOwned::to_owned)
        .or_else(|| mcp_result_text(payload))
        .or_else(|| {
            if result_is_error(payload).unwrap_or(false) {
                Some("调用失败".to_string())
            } else {
                Some("调用完成".to_string())
            }
        })
}

fn mcp_result_text(payload: &Value) -> Option<String> {
    let text = payload
        .get("result")
        .and_then(|value| value.get("Ok"))
        .and_then(|value| value.get("content"))
        .and_then(extract_text)?;
    Some(truncate_chars(&text, 300))
}

fn image_generation_message_text(payload: &Value) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(status) = payload.get("status").and_then(Value::as_str) {
        parts.push(status.to_string());
    }
    if let Some(path) = payload.get("saved_path").and_then(Value::as_str) {
        parts.push(path.to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

fn extract_image_generation_media(payload: &Value) -> Vec<ParsedMedia> {
    let mut media = extract_image_media(payload);
    if let Some(path) = payload
        .get("saved_path")
        .or_else(|| payload.get("savedPath"))
        .and_then(Value::as_str)
    {
        push_media_source(&mut media, path, None);
    }
    if let Some(result) = payload.get("result").and_then(Value::as_str) {
        let source = if result.starts_with("data:") {
            result.to_string()
        } else {
            format!("data:image/png;base64,{result}")
        };
        push_media_source(&mut media, &source, Some("image/png"));
    }
    media
}

fn extract_image_media(payload: &Value) -> Vec<ParsedMedia> {
    let mut media = Vec::new();
    collect_image_media(payload, None, &mut media);
    media
}

fn collect_image_media(value: &Value, parent_key: Option<&str>, media: &mut Vec<ParsedMedia>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_image_media(item, parent_key, media);
            }
        }
        Value::Object(map) => {
            let is_image_block = map
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.contains("image"));
            let block_mime = is_image_block
                .then(|| map.get("mime_type").or_else(|| map.get("mimeType")))
                .flatten()
                .and_then(Value::as_str);
            if is_image_block {
                if let Some(data) = map.get("data").and_then(Value::as_str) {
                    let mime = block_mime.unwrap_or("image/png");
                    let source = if data.starts_with("data:") {
                        data.to_string()
                    } else {
                        format!("data:{mime};base64,{data}")
                    };
                    push_media_source(media, &source, Some(mime));
                }
            }
            for (key, item) in map {
                if key == "image_url" {
                    if let Some(source) = item.as_str() {
                        push_media_source(media, source, block_mime);
                    }
                    continue;
                }
                if matches!(key.as_str(), "images" | "local_images") {
                    if let Some(items) = item.as_array() {
                        for source in items.iter().filter_map(Value::as_str) {
                            push_media_source(media, source, None);
                        }
                    }
                    continue;
                }
                collect_image_media(item, Some(key), media);
            }
        }
        Value::String(source)
            if parent_key
                .is_some_and(|key| matches!(key, "image_url" | "images" | "local_images")) =>
        {
            push_media_source(media, source, None);
        }
        _ => {}
    }
}

fn push_media_source(media: &mut Vec<ParsedMedia>, source: &str, mime_type: Option<&str>) {
    if source.trim().is_empty() {
        return;
    }
    let inferred_mime = mime_type.map(ToOwned::to_owned).or_else(|| {
        source
            .strip_prefix("data:")
            .and_then(|value| value.split_once(';'))
            .map(|(mime, _)| mime.to_string())
            .filter(|mime| mime.starts_with("image/"))
    });
    if let Some(existing) = media.iter_mut().find(|item| item.source == source) {
        if existing.mime_type.is_none() {
            existing.mime_type = inferred_mime;
        }
        return;
    }
    media.push(ParsedMedia {
        kind: "image".to_string(),
        mime_type: inferred_mime,
        source: source.to_string(),
    });
}

fn sub_agent_message_text(payload: &Value) -> Option<String> {
    let kind = payload
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("activity");
    let agent_path = payload
        .get("agent_path")
        .and_then(Value::as_str)
        .unwrap_or("sub-agent");
    Some(format!("{kind} · {agent_path}"))
}

fn duration_seconds(payload: &Value) -> Option<f64> {
    payload
        .get("duration_sec")
        .and_then(Value::as_f64)
        .or_else(|| {
            let duration = payload.get("duration")?;
            let secs = duration.get("secs").and_then(Value::as_f64).unwrap_or(0.0);
            let nanos = duration.get("nanos").and_then(Value::as_f64).unwrap_or(0.0);
            Some(secs + nanos / 1_000_000_000.0)
        })
}

fn result_is_error(payload: &Value) -> Option<bool> {
    payload
        .get("is_error")
        .and_then(Value::as_bool)
        .or_else(|| {
            payload
                .get("result")
                .and_then(|value| value.get("Ok"))
                .and_then(|value| value.get("isError"))
                .and_then(Value::as_bool)
        })
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, character) in value.chars().enumerate() {
        if index >= max_chars {
            output.push_str("...");
            break;
        }
        output.push(character);
    }
    output
}

fn visible_message_text(payload: &Value) -> Option<String> {
    payload.get("content").and_then(extract_text)
        .filter(|text| !is_internal_scaffold_message(text))
        .map(|text| strip_image_placeholders(&text))
        .filter(|text| !text.is_empty())
}

fn extract_response_message(payload: &Value, timestamp: Option<String>) -> Option<ParsedMessage> {
    let role = payload.get("role").and_then(Value::as_str)?;
    if matches!(role, "developer" | "system") {
        return None;
    }

    let text = payload
        .get("content")
        .and_then(extract_text)
        .map(|value| strip_image_placeholders(&value));
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
        media: extract_image_media(payload),
    })
}

fn strip_image_placeholders(text: &str) -> String {
    let mut output = String::new();
    let mut remaining = text;
    loop {
        let Some(start) = remaining.find("<image>") else {
            output.push_str(remaining);
            break;
        };
        output.push_str(&remaining[..start]);
        let after_open = &remaining[start + "<image>".len()..];
        let Some(end) = after_open.find("</image>") else {
            output.push_str(&remaining[start..]);
            break;
        };
        remaining = &after_open[end + "</image>".len()..];
    }
    output.trim().to_string()
}

fn is_internal_scaffold_message(text: &str) -> bool {
    let trimmed = text.trim_start();
    [
        "# AGENTS.md instructions",
        "<turn_aborted>",
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
    use super::{
        compact_codex_event_payload, extract_codex_message, is_internal_scaffold_message,
        tool_name_from_response_item,
    };
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

    #[test]
    fn names_new_tool_response_items_from_type() {
        assert_eq!(
            tool_name_from_response_item(&json!({"type": "tool_search_call"})).as_deref(),
            Some("tool_search")
        );
        assert_eq!(
            tool_name_from_response_item(&json!({"type": "image_generation_call"})).as_deref(),
            Some("image_generation")
        );
    }

    #[test]
    fn compacts_image_generation_without_binary_payload() {
        let payload = json!({
            "type": "image_generation_end",
            "call_id": "ig_123",
            "status": "completed",
            "revised_prompt": "large prompt",
            "result": "iVBORw0KGgoAAAANSUhEUgAA",
            "saved_path": "/tmp/image.png"
        });

        let compact = compact_codex_event_payload("event_msg", &payload);

        assert!(compact.contains("image_generation_end"));
        assert!(compact.contains("/tmp/image.png"));
        assert!(!compact.contains("iVBORw0KGgo"));
        assert!(!compact.contains("large prompt"));
    }

    #[test]
    fn reads_custom_tool_input_from_current_protocol() {
        let payload = json!({
            "type": "custom_tool_call",
            "name": "exec",
            "call_id": "call_123",
            "input": "rg --files"
        });

        let message = extract_codex_message("response_item", &payload, None)
            .expect("custom tool call should be kept");

        assert_eq!(message.text.as_deref(), Some("rg --files"));
    }

    #[test]
    fn extracts_image_blocks_without_leaking_placeholders() {
        let payload = json!({
            "type": "message",
            "role": "user",
            "content": [
                {"type": "input_text", "text": "看看这张图\n<image>large inline placeholder</image>"},
                {"type": "input_image", "image_url": "data:image/png;base64,aGVsbG8="}
            ]
        });

        let message = extract_codex_message("response_item", &payload, None)
            .expect("image message should be kept");

        assert_eq!(message.text.as_deref(), Some("看看这张图"));
        assert_eq!(message.media.len(), 1);
        assert_eq!(message.media[0].mime_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn extracts_mcp_image_content_blocks() {
        let payload = json!({
            "type": "custom_tool_call_output",
            "output": [{"type": "image", "mimeType": "image/webp", "data": "UklGRg=="}]
        });

        let message = extract_codex_message("response_item", &payload, None)
            .expect("tool output should be kept");

        assert_eq!(message.media.len(), 1);
        assert!(message.media[0]
            .source
            .starts_with("data:image/webp;base64,"));
    }

    #[test]
    fn preserves_visible_message_text_without_media_or_private_context() {
        let payload = json!({"type":"message", "role":"user", "content":[
            {"type":"input_text", "text":"Describe this image"},
            {"type":"input_image", "image_url":"data:image/png;base64,PRIVATE_IMAGE"}
        ]});
        let compact: serde_json::Value = serde_json::from_str(&compact_codex_event_payload("response_item", &payload)).unwrap();
        assert_eq!(compact["text"], "Describe this image");
        assert_eq!(compact["role"], "user");
        assert!(!compact.to_string().contains("PRIVATE_IMAGE"));
        for role in ["developer", "system"] {
            let compact = compact_codex_event_payload("response_item", &json!({
                "type":"message", "role":role, "content":[{"type":"input_text", "text":"private instructions"}]
            }));
            assert!(!compact.contains("private instructions"));
        }
        let compact = compact_codex_event_payload("event_msg", &json!({
            "type":"item_completed", "item":{"type":"UserMessage", "content":[{"type":"text", "text":"<environment_context>private</environment_context>"}]}
        }));
        assert!(!compact.contains("private"));
    }

    #[test]
    fn compacts_completed_items_with_structured_outcome() {
        let payload = json!({
            "type": "item_completed",
            "turn_id": "turn_123",
            "started_at_ms": 1000,
            "completed_at_ms": 2500,
            "item": {
                "type": "CommandExecution",
                "id": "item_123",
                "status": "failed",
                "command": "false",
                "exit_code": 1,
                "duration": {"secs": 1, "nanos": 500000000},
                "stdout": "large output that should not be stored"
            }
        });

        let compact = compact_codex_event_payload("event_msg", &payload);

        assert!(compact.contains("CommandExecution"));
        assert!(compact.contains("\"exit_code\":1"));
        assert!(!compact.contains("large output"));
    }

    #[test]
    fn compacts_current_token_usage_records() {
        let payload = json!({
            "thread_id": "thread_123",
            "turn_id": "turn_123",
            "response_id": "resp_123",
            "usage": {
                "input_tokens": 100,
                "cached_input_tokens": 40,
                "cache_write_input_tokens": 12,
                "output_tokens": 20,
                "reasoning_output_tokens": 7,
                "total_tokens": 120
            },
            "turn_token_usage": {"total_tokens": 9999},
            "thread_token_usage": {"total_tokens": 99999}
        });

        let compact = compact_codex_event_payload("token_usage_record", &payload);

        assert!(compact.contains("resp_123"));
        assert!(compact.contains("cache_write_input_tokens"));
        assert!(!compact.contains("9999"));
        assert!(!compact.contains("99999"));
        assert!(!compact.contains("thread_123"));
    }

    #[test]
    fn world_state_does_not_persist_embedded_instructions() {
        let payload = json!({
            "full": true,
            "state": {
                "model": "gpt-5.5",
                "cwd": "/tmp/project",
                "instructions": "TOP SECRET AGENTS CONTENT",
                "developer_instructions": "DO NOT STORE THIS"
            }
        });

        let compact = compact_codex_event_payload("world_state", &payload);

        assert!(compact.contains("world_state"));
        assert!(compact.contains("gpt-5.5"));
        assert!(!compact.contains("TOP SECRET"));
        assert!(!compact.contains("DO NOT STORE"));
    }

    #[test]
    fn turn_context_keeps_analytics_fields_without_private_context() {
        let payload = json!({
            "turn_id": "turn_123",
            "model": "gpt-5.5",
            "effort": "high",
            "cwd": "/tmp/project",
            "user_instructions": "PRIVATE USER INSTRUCTIONS",
            "collaboration_mode": {
                "mode": "default",
                "settings": {
                    "reasoning_effort": "high",
                    "developer_instructions": "PRIVATE DEVELOPER INSTRUCTIONS"
                }
            }
        });

        let compact = compact_codex_event_payload("turn_context", &payload);

        assert!(compact.contains("turn_123"));
        assert!(compact.contains("gpt-5.5"));
        assert!(compact.contains("reasoning_effort"));
        assert!(!compact.contains("PRIVATE USER"));
        assert!(!compact.contains("PRIVATE DEVELOPER"));
    }

    #[test]
    fn compacts_current_terminal_events_without_large_details() {
        let patch_payload = json!({
            "type": "patch_apply_end",
            "call_id": "call_patch",
            "success": true,
            "status": "completed",
            "changes": "very large unified diff"
        });
        let search_payload = json!({
            "type": "web_search_end",
            "call_id": "call_search",
            "query": "Codex image input",
            "action": {"type": "search", "queries": ["Codex image input"]},
            "results": "large search response"
        });

        let patch = compact_codex_event_payload("event_msg", &patch_payload);
        let search = compact_codex_event_payload("event_msg", &search_payload);

        assert!(patch.contains("call_patch"));
        assert!(!patch.contains("unified diff"));
        assert!(search.contains("Codex image input"));
        assert!(!search.contains("large search response"));
    }
}
