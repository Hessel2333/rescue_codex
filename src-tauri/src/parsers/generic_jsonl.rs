use crate::models::parser::{
    ParseContext, ParseResult, ParseWarning, ParsedRawEvent, ParserTarget,
};
use crate::parsers::{finalize_session, generic_message_from_value, SessionSeed, SourceParser};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub struct GenericJsonlParser;

impl SourceParser for GenericJsonlParser {
    fn key(&self) -> &'static str {
        "generic_jsonl"
    }

    fn version(&self) -> &'static str {
        "1"
    }

    fn supports(&self, target: &ParserTarget) -> u8 {
        if target.extension == "jsonl" {
            60
        } else {
            0
        }
    }

    fn parse(&self, target: &ParserTarget, ctx: &ParseContext) -> anyhow::Result<ParseResult> {
        let file = File::open(&ctx.abs_path)?;
        let reader = BufReader::new(file);

        let mut warnings = Vec::new();
        let mut events = Vec::new();
        let mut messages = Vec::new();
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
                .map(ToOwned::to_owned)
                .or_else(|| {
                    value
                        .get("updated_at")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .or_else(|| {
                    value
                        .get("created_at")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                });
            if timestamp.is_some() {
                last_ts = timestamp.clone();
            }

            events.push(ParsedRawEvent {
                seq: events.len() as i64 + 1,
                ts: timestamp.clone(),
                outer_type: value
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("jsonl_item")
                    .to_string(),
                inner_type: value
                    .get("payload")
                    .and_then(|payload| payload.get("type"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                payload_json: serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()),
                warning_code: None,
            });

            if let Some(message) = generic_message_from_value(&value, timestamp) {
                messages.push(message);
            }
        }

        let seed = SessionSeed {
            id: None,
            thread_title: target
                .abs_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned),
            cwd: None,
            originator: Some("Generic JSONL Import".to_string()),
            source: Some("manual".to_string()),
            model_provider: None,
            cli_version: None,
            started_at: None,
            updated_at: last_ts,
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
