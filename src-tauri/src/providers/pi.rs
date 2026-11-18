use super::{MAX_LINE_BYTES, MAX_SESSION_BYTES, open_jsonl_reader, read_bounded_line};
use crate::domain::{AppError, AttachmentRef, Message, ParsedSession, Result, SessionSummary};
use serde_json::Value;
use std::io::Read;

pub fn parse(summary: &SessionSummary) -> Result<ParsedSession> {
    let path = std::path::Path::new(&summary.container_path);
    if path.metadata()?.len() > MAX_SESSION_BYTES as u64 {
        return Err(AppError::Unsupported("会话文件超过安全读取上限".into()));
    }
    let mut reader = open_jsonl_reader(path)?;
    let mut header_line = Vec::new();
    read_bounded_line(&mut reader, &mut header_line, MAX_LINE_BYTES)?;
    let header: Value = serde_json::from_slice(&header_line)?;
    if header.get("type").and_then(Value::as_str) != Some("session")
        || header.get("id").and_then(Value::as_str) != Some(summary.source_session_id.as_str())
        || !matches!(header.get("version").and_then(Value::as_u64), Some(1..=3))
    {
        return Err(AppError::Unsupported(
            "Pi session header or version is unsupported".into(),
        ));
    }
    let mut messages = Vec::new();
    let mut line = Vec::new();
    loop {
        let count = read_bounded_line(&mut reader, &mut line, MAX_LINE_BYTES)?;
        if count == 0 {
            break;
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let event: Value = serde_json::from_slice(&line)?;
        if event.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        let Some(value) = event.get("message") else {
            continue;
        };
        let role = value
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let content = value
            .get("content")
            .map(sanitize_content)
            .unwrap_or(Value::Null);
        let attachments = collect_attachments(&content);
        messages.push(Message {
            role,
            timestamp: event
                .get("timestamp")
                .and_then(Value::as_str)
                .map(str::to_owned),
            content,
            attachments,
        });
    }
    let mut probe = [0u8; 1];
    if reader.read(&mut probe)? > 0 {
        return Err(AppError::Unsupported("解压会话超过最大安全读取量".into()));
    }
    Ok(ParsedSession {
        summary: summary.clone(),
        messages,
    })
}

fn sanitize_content(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().filter_map(|item| {
            if item.get("type").and_then(Value::as_str) == Some("image") {
                return Some(serde_json::json!({ "type": "image", "url": item.get("url").cloned().unwrap_or(Value::Null) }));
            }
            if item.get("type").and_then(Value::as_str) == Some("text") {
                return Some(serde_json::json!({ "type": "text", "text": item.get("text").cloned().unwrap_or(Value::Null) }));
            }
            let mut safe = item.clone();
            if let Some(object) = safe.as_object_mut() {
                object.remove("data");
                object.remove("base64");
            }
            Some(safe)
        }).collect()),
        other => other.clone(),
    }
}

fn collect_attachments(value: &Value) -> Vec<AttachmentRef> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let kind = item.get("type")?.as_str()?;
            match kind {
                "image" => Some(AttachmentRef {
                    kind: "image".into(),
                    reference: item
                        .get("url")
                        .or_else(|| item.get("data"))?
                        .as_str()?
                        .to_owned(),
                }),
                "text" => None,
                other => Some(AttachmentRef {
                    kind: other.to_owned(),
                    reference: item
                        .get("path")
                        .or_else(|| item.get("url"))?
                        .as_str()?
                        .to_owned(),
                }),
            }
        })
        .collect()
}
