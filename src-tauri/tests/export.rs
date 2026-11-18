use session_studio_lib::domain::*;
use session_studio_lib::providers::{export_session, messages_markdown};
use session_studio_lib::storage::Store;

fn session(content: &str) -> ParsedSession {
    ParsedSession {
        summary: SessionSummary {
            id: "pi:id".into(),
            source: SourceId::Pi,
            source_session_id: "id".into(),
            source_root: "/fixture".into(),
            container_path: "/fixture/id.jsonl".into(),
            container_id: "container".into(),
            title: Some("Synthetic <title>".into()),
            project_path: Some("/project".into()),
            message_count: Some(1),
            last_activity_unix: Some(123),
            modified_at_unix: 123,
            fingerprint: "fp".into(),
            candidate_reasons: vec![],
        },
        messages: vec![Message {
            role: "tool_result".into(),
            timestamp: Some("2026-01-01T00:00:00Z".into()),
            content: serde_json::json!({ "text": content }),
            attachments: vec![AttachmentRef {
                kind: "image".into(),
                reference: "file:///tmp/image.png".into(),
            }],
        }],
    }
}

#[test]
fn markdown_export_preserves_tool_text_timestamp_and_attachment_reference() {
    let output = messages_markdown(&session("synthetic tool output"));
    assert!(output.contains("tool_result"));
    assert!(output.contains("2026-01-01T00:00:00Z"));
    assert!(output.contains("synthetic tool output"));
    assert!(output.contains("file:///tmp/image.png"));
}

#[test]
fn exports_write_only_into_the_user_selected_destination() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let session = session("synthetic");
    export_session(&store, &session, "json", temp.path()).unwrap();
    let exported: serde_json::Value =
        serde_json::from_slice(&std::fs::read(temp.path().join("id.json")).unwrap()).unwrap();
    assert_eq!(exported["messages"][0]["role"], "tool_result");
    assert!(export_session(&store, &session, "html", temp.path()).is_err());
}
