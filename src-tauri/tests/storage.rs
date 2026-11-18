use session_studio_lib::domain::*;
use session_studio_lib::storage::Store;

fn summary(id: &str) -> SessionSummary {
    SessionSummary {
        id: id.into(),
        source: SourceId::Codex,
        source_session_id: "thread".into(),
        source_root: "/synthetic".into(),
        container_path: "/synthetic/rollout.jsonl".into(),
        container_id: "container".into(),
        title: None,
        project_path: None,
        message_count: None,
        last_activity_unix: None,
        modified_at_unix: 100,
        fingerprint: "fingerprint".into(),
        candidate_reasons: vec![CandidateReason::Unassociated],
    }
}

fn status() -> SourceStatus {
    SourceStatus {
        source: SourceId::Codex,
        root: Some("/synthetic".into()),
        default_root: Some("/default/codex".into()),
        uses_default_root: false,
        health: SourceHealth::Ready,
        last_scan_unix: Some(100),
        session_count: 1,
        error: None,
        cli_available: false,
    }
}

#[test]
fn legacy_selected_roots_migrate_to_custom_root_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE sources (
            source TEXT PRIMARY KEY,
            root TEXT,
            health TEXT NOT NULL,
            last_scan_unix INTEGER,
            session_count INTEGER NOT NULL,
            error TEXT,
            cli_available INTEGER NOT NULL
        );
        INSERT INTO sources(source, root, health, session_count, cli_available)
        VALUES ('codex', '/previously/selected/codex', 'ready', 0, 1);",
        )
        .unwrap();
    drop(connection);

    let db = Store::open(&path).unwrap();

    assert_eq!(
        db.custom_root(SourceId::Codex).unwrap().as_deref(),
        Some("/previously/selected/codex")
    );
    let source = db.sources().unwrap().remove(0);
    assert!(!source.uses_default_root);
}

#[test]
fn default_root_scan_uses_current_environment_path_in_returned_status() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("default-status.sqlite");
    let mut db = Store::open(&path).unwrap();
    let mut default_status = status();
    default_status.root = Some("/previous-default/codex".into());
    default_status.default_root = Some("/previous-default/codex".into());
    default_status.uses_default_root = true;
    db.complete_scan(&default_status, &[]).unwrap();
    let returned = db.sources().unwrap().remove(0);
    assert!(returned.uses_default_root);
    assert_eq!(returned.root, returned.default_root);
}

#[test]
fn reopening_a_default_root_scan_does_not_migrate_it_as_custom() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("default.sqlite");
    let mut db = Store::open(&path).unwrap();
    let mut default_status = status();
    default_status.root = Some("/default/codex".into());
    default_status.default_root = Some("/default/codex".into());
    default_status.uses_default_root = true;
    db.complete_scan(&default_status, &[]).unwrap();
    drop(db);

    let db = Store::open(&path).unwrap();
    assert_eq!(db.custom_root(SourceId::Codex).unwrap(), None);
}

#[test]
fn custom_roots_are_persisted_separately_from_scan_status() {
    let mut db = Store::open_in_memory().unwrap();
    db.set_custom_root(SourceId::Codex, "/custom/codex")
        .unwrap();
    assert_eq!(
        db.custom_root(SourceId::Codex).unwrap().as_deref(),
        Some("/custom/codex")
    );
    db.clear_custom_root(SourceId::Codex).unwrap();
    assert_eq!(db.custom_root(SourceId::Codex).unwrap(), None);
    assert!(db.sessions().unwrap().is_empty());
    assert!(db.sources().unwrap().is_empty());
}

#[test]
fn changing_custom_root_clears_only_that_sources_index_and_scan_status() {
    let mut db = Store::open_in_memory().unwrap();
    db.complete_scan(&status(), &[summary("codex-old")])
        .unwrap();
    let mut pi_status = status();
    pi_status.source = SourceId::Pi;
    pi_status.root = Some("/synthetic/pi".into());
    let mut pi_session = summary("pi-kept");
    pi_session.source = SourceId::Pi;
    pi_session.source_root = "/synthetic/pi".into();
    pi_session.container_path = "/synthetic/pi/session.jsonl".into();
    db.complete_scan(&pi_status, &[pi_session]).unwrap();

    db.set_custom_root(SourceId::Codex, "/custom/codex")
        .unwrap();

    let sessions = db.sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "pi-kept");
    assert_eq!(
        db.custom_root(SourceId::Codex).unwrap().as_deref(),
        Some("/custom/codex")
    );
    assert_eq!(db.sources().unwrap().len(), 1);
}

#[test]
fn session_queries_exclude_indexes_outside_the_current_source_root() {
    let mut db = Store::open_in_memory().unwrap();
    let mut old_status = status();
    old_status.default_root = Some("/previous/default/codex".into());
    old_status.uses_default_root = true;
    old_status.root = old_status.default_root.clone();
    let mut old_session = summary("old-default");
    old_session.source_root = "/previous/default/codex".into();
    db.complete_scan(&old_status, &[old_session]).unwrap();

    let sessions = db.sessions_with_candidates(100).unwrap();
    assert!(sessions.is_empty());
    let source = db.sources().unwrap().remove(0);
    assert!(source.uses_default_root);
    assert_eq!(source.root, source.default_root);
    assert_eq!(source.health, SourceHealth::NotScanned);
    assert_eq!(source.session_count, 0);
}

#[test]
fn scan_replaces_only_its_source_and_keeps_stable_ids() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let mut db = Store::open(&path).unwrap();
    db.complete_scan(&status(), &[summary("one")]).unwrap();
    let mut updated = summary("one");
    updated.title = Some("Changed metadata".into());
    db.complete_scan(&status(), &[updated]).unwrap();
    drop(db);
    let db = Store::open(&path).unwrap();
    let sessions = db.sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].title.as_deref(), Some("Changed metadata"));
    assert_eq!(db.sources().unwrap()[0].health, SourceHealth::Ready);
}

#[test]
fn changing_authorized_root_replaces_previous_source_index() {
    let mut db = Store::open_in_memory().unwrap();
    db.complete_scan(&status(), &[summary("old")]).unwrap();
    let mut next_status = status();
    next_status.root = Some("/new-root".into());
    let mut next = summary("new");
    next.source_root = "/new-root".into();
    next.container_path = "/new-root/session.jsonl".into();
    db.complete_scan(&next_status, &[next]).unwrap();
    let sessions = db.sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "new");
    assert_eq!(sessions[0].source_root, "/new-root");
}

#[test]
fn failed_status_does_not_destroy_previous_index() {
    let mut db = Store::open_in_memory().unwrap();
    db.complete_scan(&status(), &[summary("one")]).unwrap();
    let mut failed = status();
    failed.health = SourceHealth::PermissionDenied;
    failed.error = Some("denied".into());
    db.save_source(&failed).unwrap();
    assert_eq!(db.sessions().unwrap().len(), 1);
    assert_eq!(
        db.sources().unwrap()[0].health,
        SourceHealth::PermissionDenied
    );
}

#[test]
fn clearing_metadata_preserves_analysis_and_custom_root_configuration() {
    let mut db = Store::open_in_memory().unwrap();
    db.complete_scan(&status(), &[summary("one")]).unwrap();
    let task = AnalysisTask {
        id: "task".into(),
        source: SourceId::Pi,
        status: "failed".into(),
        created_at_unix: 12,
        input: "synthetic task input".into(),
        stdout: "partial".into(),
        stderr: "failure".into(),
        exit_code: Some(1),
        error: Some("exited".into()),
    };
    db.save_task(&task).unwrap();
    db.clear_index().unwrap();
    assert!(db.sessions().unwrap().is_empty());
    assert_eq!(db.tasks().unwrap()[0].stdout, "partial");
    let source = db.sources().unwrap().remove(0);
    assert_eq!(source.root.as_deref(), Some("/synthetic"));
    assert!(!source.uses_default_root);
    assert_eq!(source.health, SourceHealth::NotScanned);
}

#[test]
fn candidate_query_recomputes_missing_project_unassociated_and_ninety_day_signals() {
    let mut db = Store::open_in_memory().unwrap();
    let mut missing = summary("missing");
    missing.project_path = Some("/definitely-not-a-session-studio-project".into());
    missing.last_activity_unix = Some(100);
    missing.modified_at_unix = 100;
    let mut unassociated = summary("unassociated");
    unassociated.project_path = None;
    unassociated.last_activity_unix = Some(100);
    unassociated.modified_at_unix = 100;
    let mut fresh = summary("fresh");
    fresh.project_path = Some("/".into());
    fresh.last_activity_unix = Some(1_000);
    fresh.modified_at_unix = 1_000;
    db.complete_scan(&status(), &[missing, unassociated, fresh])
        .unwrap();

    let candidates = db.candidate_sessions(100 + 90 * 86_400).unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].candidate_reasons,
        vec![
            CandidateReason::MissingProject,
            CandidateReason::Inactive90Days
        ]
    );
    assert_eq!(
        candidates[1].candidate_reasons,
        vec![
            CandidateReason::Unassociated,
            CandidateReason::Inactive90Days
        ]
    );
}

#[test]
fn session_schema_contains_no_message_or_body_column() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    Store::open(&path).unwrap();
    let connection = rusqlite::Connection::open(path).unwrap();
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info('sessions')")
        .unwrap();
    let names: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();
    assert_eq!(names, vec!["id", "source", "container_path", "metadata"]);
}
