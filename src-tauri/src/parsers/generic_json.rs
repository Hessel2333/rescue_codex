use crate::models::parser::{
    ParseContext, ParseResult, ParseWarning, ParsedRawEvent, ParserTarget,
};
use crate::parsers::{
    extract_text, finalize_session, generic_message_from_value, SessionSeed, SourceParser,
};
use serde_json::Value;
use std::fs;

pub struct GenericJsonParser;

impl SourceParser for GenericJsonParser {
    fn key(&self) -> &'static str {
        "generic_json"
    }

    fn version(&self) -> &'static str {
        "1"
    }

    fn supports(&self, target: &ParserTarget) -> u8 {
        if target.extension == "json" {
            70
        } else {
            5
        }
    }

    fn parse(&self, target: &ParserTarget, ctx: &ParseContext) -> anyhow::Result<ParseResult> {
        let content = fs::read_to_string(&ctx.abs_path)?;
        let value: Value = serde_json::from_str(&content)?;
        let mut warnings = Vec::<ParseWarning>::new();
        let mut events = Vec::<ParsedRawEvent>::new();
        let mut messages = Vec::new();

        match &value {
            Value::Array(items) => {
                for item in items {
                    let ts = item
                        .get("timestamp")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            item.get("updated_at")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned)
                        });
                    events.push(ParsedRawEvent {
                        seq: events.len() as i64 + 1,
                        ts: ts.clone(),
                        outer_type: item
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("json_array_item")
                            .to_string(),
                        inner_type: None,
                        payload_json: serde_json::to_string(item)
                            .unwrap_or_else(|_| "null".to_string()),
                        warning_code: None,
                    });
                    if let Some(message) = generic_message_from_value(item, ts) {
                        messages.push(message);
                    }
                }
            }
            Value::Object(map) => {
                events.push(ParsedRawEvent {
                    seq: 1,
                    ts: map
                        .get("updated_at")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            map.get("timestamp")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned)
                        }),
                    outer_type: map
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("json_object")
                        .to_string(),
                    inner_type: None,
                    payload_json: serde_json::to_string(&value)
                        .unwrap_or_else(|_| "null".to_string()),
                    warning_code: None,
                });

                if let Some(array) = map.get("messages").and_then(Value::as_array) {
                    for item in array {
                        let ts = item
                            .get("timestamp")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .or_else(|| {
                                map.get("updated_at")
                                    .and_then(Value::as_str)
                                    .map(ToOwned::to_owned)
                            });
                        if let Some(message) = generic_message_from_value(item, ts) {
                            messages.push(message);
                        }
                    }
                } else if let Some(message) = generic_message_from_value(
                    &value,
                    events.first().and_then(|event| event.ts.clone()),
                ) {
                    messages.push(message);
                } else if let Some(text) = map.get("content").and_then(extract_text) {
                    messages.push(crate::models::parser::ParsedMessage {
                        turn_id: None,
                        role: map
                            .get("role")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        kind: "message".to_string(),
                        text: Some(text),
                        ts: events.first().and_then(|event| event.ts.clone()),
                        tool_name: None,
                        phase: None,
                        meta_json: serde_json::to_string(&value)
                            .unwrap_or_else(|_| "{}".to_string()),
                    });
                }
            }
            _ => warnings.push(ParseWarning {
                severity: "warning".to_string(),
                code: "unsupported_json_shape".to_string(),
                message: "JSON 顶层不是对象或数组，已按原始事件保存。".to_string(),
                line_no: None,
                raw_excerpt: None,
            }),
        }

        let updated_at = events.iter().rev().find_map(|event| event.ts.clone());
        let seed = SessionSeed {
            id: None,
            thread_title: target
                .abs_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned),
            cwd: None,
            originator: Some("Generic JSON Import".to_string()),
            source: Some("manual".to_string()),
            model_provider: None,
            cli_version: None,
            started_at: None,
            updated_at,
        };

        let session = finalize_session(seed, ctx, events.len(), &mut messages, &warnings);

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
