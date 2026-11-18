use super::{MAX_LINE_BYTES, MAX_SESSION_BYTES, open_jsonl_reader, read_bounded_line};
use crate::domain::{AppError, AttachmentRef, Message, ParsedSession, Result, SessionSummary};
use serde_json::Value;
use std::io::Read;
use std::path::Path;

pub fn parse(summary: &SessionSummary) -> Result<ParsedSession> {
    let path = Path::new(&summary.container_path);
    if !path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .ends_with(".zst")
        && path.metadata()?.len() > MAX_SESSION_BYTES as u64
    {
        return Err(AppError::Unsupported(
            "Codex rollout exceeds the safety limit".into(),
        ));
    }
    let mut reader = open_jsonl_reader(path)?;
    let mut line = Vec::new();
    let mut metadata_seen = false;
    let mut messages = Vec::new();
    loop {
        let count = read_bounded_line(&mut reader, &mut line, MAX_LINE_BYTES)?;
        if count == 0 {
            break;
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let record: Value = serde_json::from_slice(&line)?;
        let kind = record.get("type").and_then(Value::as_str).unwrap_or("");
        let payload = record.get("payload").unwrap_or(&record);
        match kind {
            "session_meta" => {
                let id = payload
                    .get("id")
                    .or_else(|| payload.get("thread_id"))
                    .and_then(Value::as_str);
                if id != Some(summary.source_session_id.as_str()) {
                    return Err(AppError::Unsupported(
                        "Codex thread ID does not match index".into(),
                    ));
                }
                metadata_seen = true;
            }
            "response_item" => {
                if let Some(message) =
                    parse_response_item(payload, record.get("timestamp").and_then(Value::as_str))
                {
                    messages.push(message);
                }
            }
            _ => {}
        }
    }
    let mut probe = [0u8; 1];
    if reader.read(&mut probe)? > 0 {
        return Err(AppError::Unsupported(
            "Decompressed Codex rollout exceeds the safety limit".into(),
        ));
    }
    if !metadata_seen {
        return Err(AppError::Unsupported(
            "Codex rollout is missing session_meta".into(),
        ));
    }
    Ok(ParsedSession {
        summary: summary.clone(),
        messages,
    })
}

fn parse_response_item(payload: &Value, timestamp: Option<&str>) -> Option<Message> {
    let kind = payload.get("type")?.as_str()?;
    let role;
    let content;
    match kind {
        "message" => {
            role = payload.get("role")?.as_str()?.to_owned();
            content = Value::Array(payload.get("content")?.as_array()?.iter().filter_map(|part| {
                let item_kind = part.get("type")?.as_str()?;
                match item_kind {
                    "input_text" | "output_text" | "text" => Some(part.get("text").cloned().unwrap_or(Value::Null)),
                    "image" | "input_image" => Some(serde_json::json!({ "type": "image", "reference": part.get("image_url").or_else(|| part.get("url")).cloned().unwrap_or(Value::Null) })),
                    _ => None,
                }
            }).collect());
        }
        "function_call" | "custom_tool_call" | "computer_call" => {
            role = "tool_call".to_owned();
            content = payload.clone();
        }
        "function_call_output" | "custom_tool_call_output" | "computer_call_output" => {
            role = "tool_result".to_owned();
            content = payload.clone();
        }
        "reasoning" => return None,
        _ => return None,
    }
    let attachments = content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(AttachmentRef {
                kind: "image".into(),
                reference: item.get("reference")?.as_str()?.to_owned(),
            })
        })
        .collect();
    Some(Message {
        role,
        timestamp: timestamp.map(str::to_owned),
        content,
        attachments,
    })
}
