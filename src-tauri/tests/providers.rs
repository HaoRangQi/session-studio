use session_studio_lib::domain::*;
use session_studio_lib::providers::{configure_root, parse_session, scan_source};
use session_studio_lib::storage::Store;
use std::fs;
use std::path::Path;

fn fixture(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn missing_default_directory_is_reported_without_permission_or_format_failure() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("not-created");
    let mut store = Store::open_in_memory().unwrap();
    let status = scan_source(&mut store, SourceId::Pi, &missing).unwrap();
    assert_eq!(status.health, SourceHealth::Missing);
    assert!(status.error.is_some());
}

#[cfg(unix)]
#[test]
fn symlinked_custom_root_keeps_the_configured_path_as_index_identity() {
    let temp = tempfile::tempdir().unwrap();
    let real_root = temp.path().join("real-pi");
    fixture(
        &real_root.join("--project--/123_symlink.jsonl"),
        "{\"type\":\"session\",\"version\":3,\"id\":\"symlink\",\"cwd\":\"/project\"}\n",
    );
    let displayed_root = temp.path().join("default-pi");
    std::os::unix::fs::symlink(&real_root, &displayed_root).unwrap();
    let mut store = Store::open_in_memory().unwrap();
    configure_root(&mut store, SourceId::Pi, &displayed_root).unwrap();

    let status = scan_source(&mut store, SourceId::Pi, &displayed_root).unwrap();
    let session = store.sessions().unwrap().remove(0);

    assert_eq!(
        status.root.as_deref(),
        Some(displayed_root.to_str().unwrap())
    );
    assert_eq!(session.source_root, displayed_root.to_string_lossy());
}

#[test]
fn pi_scan_indexes_header_only_and_detail_loads_messages_on_demand() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("pi");
    fixture(
        &root.join("--project--/123_abc.jsonl"),
        "{\"type\":\"session\",\"version\":3,\"id\":\"abc\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"cwd\":\"/project\"}\n{\"type\":\"message\",\"id\":\"1\",\"parentId\":null,\"timestamp\":\"2026-01-01T00:00:01Z\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"synthetic body\"}]}}\n",
    );
    let mut store = Store::open_in_memory().unwrap();
    configure_root(&mut store, SourceId::Pi, &root).unwrap();
    let status = scan_source(&mut store, SourceId::Pi, &root).unwrap();
    assert_eq!(status.session_count, 1);
    let index = store.sessions().unwrap();
    assert!(
        !serde_json::to_string(&index[0])
            .unwrap()
            .contains("synthetic body")
    );
    let detail = parse_session(&store, &index[0].id).unwrap();
    assert_eq!(detail.messages.len(), 1);
    assert!(
        detail.messages[0]
            .content
            .to_string()
            .contains("synthetic body")
    );
}

#[test]
fn codex_session_meta_is_supported_and_filename_is_container_identity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("codex");
    fixture(
        &root.join("sessions/2026/11/08/rollout-2026-11-08T00-00-00-thread-1.jsonl"),
        "{\"timestamp\":\"2026-11-08T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"thread-1\",\"cwd\":\"/project\"}}\n{\"timestamp\":\"2026-11-08T00:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"synthetic request\"}]}}\n{\"timestamp\":\"2026-11-08T00:00:02Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"synthetic answer\"}]}}\n",
    );
    let mut store = Store::open_in_memory().unwrap();
    configure_root(&mut store, SourceId::Codex, &root).unwrap();
    let status = scan_source(&mut store, SourceId::Codex, &root).unwrap();
    assert_eq!(status.session_count, 1);
    let session = store.sessions().unwrap().remove(0);
    let detail = parse_session(&store, &session.id).unwrap();
    assert_eq!(detail.messages.len(), 2);
    assert_eq!(detail.messages[1].role, "assistant");
    assert!(
        detail.messages[1]
            .content
            .to_string()
            .contains("synthetic answer")
    );
}

#[test]
fn codex_zstd_rollout_can_be_scanned_and_parsed_with_a_decompression_limit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("codex");
    let path = root.join("archived_sessions/2026/rollout-thread-zst.jsonl.zst");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let content = "{\"type\":\"session_meta\",\"payload\":{\"id\":\"thread-zst\",\"cwd\":\"/project\"}}\n{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"compressed sample\"}]}}\n";
    let file = fs::File::create(&path).unwrap();
    let mut encoder = zstd::Encoder::new(file, 1).unwrap();
    std::io::Write::write_all(&mut encoder, content.as_bytes()).unwrap();
    encoder.finish().unwrap();
    let mut store = Store::open_in_memory().unwrap();
    configure_root(&mut store, SourceId::Codex, &root).unwrap();
    let status = scan_source(&mut store, SourceId::Codex, &root).unwrap();
    assert_eq!(status.session_count, 1);
    let session = store.sessions().unwrap().remove(0);
    let parsed = parse_session(&store, &session.id).unwrap();
    assert!(
        parsed.messages[0]
            .content
            .to_string()
            .contains("compressed sample")
    );
}

#[test]
fn pi_attachment_payload_bytes_are_not_retained_in_message_content() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("pi");
    fixture(
        &root.join("--project--/123_img.jsonl"),
        "{\"type\":\"session\",\"version\":3,\"id\":\"img\",\"cwd\":\"/project\"}\n{\"type\":\"message\",\"timestamp\":\"t\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"image\",\"data\":\"SYNTHETIC_BASE64_SECRET\"}]}}\n",
    );
    let mut store = Store::open_in_memory().unwrap();
    configure_root(&mut store, SourceId::Pi, &root).unwrap();
    let status = scan_source(&mut store, SourceId::Pi, &root).unwrap();
    let session = store.sessions().unwrap().remove(0);
    let parsed = parse_session(&store, &session.id).unwrap();
    assert_eq!(status.session_count, 1);
    assert!(
        !parsed.messages[0]
            .content
            .to_string()
            .contains("SYNTHETIC_BASE64_SECRET")
    );
}

#[test]
fn claude_and_deepseek_stay_needs_validation_instead_of_fake_empty_success() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("unknown");
    fs::create_dir_all(&root).unwrap();
    let mut store = Store::open_in_memory().unwrap();
    for source in [SourceId::ClaudeCode, SourceId::DeepseekHarness] {
        let status = scan_source(&mut store, source, &root).unwrap();
        assert_eq!(status.health, SourceHealth::NeedsValidation);
        assert!(status.error.is_some());
    }
}

#[test]
fn malformed_jsonl_is_an_error_and_not_an_empty_scan() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("pi");
    fixture(&root.join("--project--/session.jsonl"), "not-json\n");
    let mut store = Store::open_in_memory().unwrap();
    assert!(scan_source(&mut store, SourceId::Pi, &root).is_err());
}
