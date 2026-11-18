use session_studio_lib::domain::*;
use session_studio_lib::providers::{cleanup_preview, execute_cleanup_with};
use session_studio_lib::storage::Store;
use std::path::Path;

fn setup(
    root: &str,
    path: &str,
    activity_reliable: bool,
    cwd: Option<&str>,
    running: bool,
    project: Option<&str>,
) -> (Store, String) {
    let mut store = Store::open_in_memory().unwrap();
    let status = SourceStatus {
        source: SourceId::Pi,
        root: Some(root.into()),
        default_root: Some("/default/pi".into()),
        uses_default_root: false,
        health: SourceHealth::Ready,
        last_scan_unix: Some(1),
        session_count: 1,
        error: None,
        cli_available: true,
    };
    let session = SessionSummary {
        id: "pi:one".into(),
        source: SourceId::Pi,
        source_session_id: "one".into(),
        source_root: root.into(),
        container_path: path.into(),
        container_id: "container".into(),
        title: None,
        project_path: project.map(str::to_owned),
        message_count: None,
        last_activity_unix: None,
        modified_at_unix: 1,
        fingerprint: "x".into(),
        candidate_reasons: vec![],
    };
    store.complete_scan(&status, &[session]).unwrap();
    let records = if running {
        vec![ActivityRecord {
            pid: 5,
            source: SourceId::Pi,
            project_path: cwd.map(str::to_owned),
            started_at_unix: Some(1),
            state: "running".into(),
            child_pids: vec![],
        }]
    } else {
        vec![]
    };
    store
        .save_activity(activity_reliable, &records, None)
        .unwrap();
    (store, "pi:one".into())
}

#[test]
fn unreliable_or_indeterminate_activity_blocks_cleanup() {
    for (reliable, cwd, project) in [
        (false, None, Some("/p")),
        (true, None, Some("/p")),
        (true, Some("/elsewhere"), None),
    ] {
        let (store, id) = setup(
            "/synthetic",
            "/synthetic/one.jsonl",
            reliable,
            cwd,
            true,
            project,
        );
        let preview = cleanup_preview(&store, &[id]).unwrap();
        assert_eq!(preview.containers[0].activity, ActivityCertainty::Unknown);
    }
}

#[test]
fn active_process_under_project_blocks_container_trash() {
    let (store, id) = setup(
        "/synthetic",
        "/synthetic/one.jsonl",
        true,
        Some("/project/subdir"),
        true,
        Some("/project"),
    );
    let preview = cleanup_preview(&store, &[id]).unwrap();
    assert!(!preview.containers[0].activity.allows_cleanup());
}

#[test]
fn same_source_process_without_session_identity_is_indeterminate_even_elsewhere() {
    let (store, _) = setup(
        "/synthetic",
        "/synthetic/one.jsonl",
        true,
        Some("/unrelated"),
        true,
        Some("/project"),
    );
    let (certainty, _) = store.cleanup_safety("/synthetic/one.jsonl").unwrap();
    assert_eq!(certainty, ActivityCertainty::Unknown);
}

#[test]
fn missing_container_is_reported_without_calling_trash() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing.jsonl");
    let root = temp.path().to_string_lossy().into_owned();
    let path = missing.to_string_lossy().into_owned();
    let (mut store, id) = setup(&root, &path, true, None, false, Some("/project"));
    let preview = cleanup_preview(&store, &[id]).unwrap();
    store.save_cleanup_preview(&preview).unwrap();
    let mut called = false;
    let result = execute_cleanup_with(&store, &preview.token, |_| {
        called = true;
        Ok(())
    })
    .unwrap();
    assert!(!called);
    assert!(!result[0].success);
    assert!(result[0].error.is_some());
}

#[test]
fn approved_inactive_container_reaches_trash_boundary_without_side_effect_in_test() {
    let temp = tempfile::tempdir().unwrap();
    let container = temp.path().join("session.jsonl");
    std::fs::write(&container, "synthetic fixture").unwrap();
    let root = temp.path().to_string_lossy().into_owned();
    let path = container.to_string_lossy().into_owned();
    let (mut store, id) = setup(&root, &path, true, None, false, Some("/project"));
    let preview = cleanup_preview(&store, &[id]).unwrap();
    store.save_cleanup_preview(&preview).unwrap();
    let mut approved_path = None;
    let result = execute_cleanup_with(&store, &preview.token, |path: &Path| {
        approved_path = Some(path.to_path_buf());
        Ok(())
    })
    .unwrap();
    assert!(result[0].success);
    assert_eq!(
        approved_path.as_deref(),
        Some(std::fs::canonicalize(&container).unwrap().as_path())
    );
    assert!(container.exists());
}
