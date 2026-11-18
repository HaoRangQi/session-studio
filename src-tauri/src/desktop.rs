use crate::domain::{
    ActivitySnapshot, AnalysisTask, AppError, ExportFormat, ParsedSession, Result, SessionSummary,
    SourceId, SourceStatus, now_unix,
};
use crate::providers;
use crate::storage::Store;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};

struct AppState(Mutex<Store>);

fn lock<'a>(state: &'a State<'_, AppState>) -> Result<std::sync::MutexGuard<'a, Store>> {
    state
        .0
        .lock()
        .map_err(|_| AppError::Invalid("本地数据库锁不可用".into()))
}

#[tauri::command]
fn get_sources(state: State<'_, AppState>) -> std::result::Result<Vec<SourceStatus>, String> {
    let store = lock(&state).map_err(|e| e.to_string())?;
    let mut known = store.sources().map_err(|e| e.to_string())?;
    for source in SourceId::ALL {
        if !known.iter().any(|item| item.source == source) {
            let mut status = providers::initial_status(source, None);
            if let Some(root) = store.custom_root(source).map_err(|e| e.to_string())? {
                status.root = Some(root);
                status.uses_default_root = false;
            }
            status.cli_available = providers::executable_available(source.executable());
            known.push(status);
        }
    }
    known.sort_by_key(|status| {
        SourceId::ALL
            .iter()
            .position(|source| source == &status.source)
            .unwrap_or(usize::MAX)
    });
    Ok(known)
}

#[tauri::command]
fn set_source_root(
    state: State<'_, AppState>,
    source: SourceId,
    root: String,
) -> std::result::Result<SourceStatus, String> {
    let mut store = lock(&state).map_err(|e| e.to_string())?;
    providers::configure_root(&mut store, source, Path::new(&root)).map_err(|e| e.to_string())
}

#[tauri::command]
fn reset_source_root(
    state: State<'_, AppState>,
    source: SourceId,
) -> std::result::Result<SourceStatus, String> {
    let mut store = lock(&state).map_err(|e| e.to_string())?;
    store
        .reset_source_root(source)
        .map_err(|error| error.to_string())?;
    let mut status = providers::initial_status(source, None);
    status.cli_available = providers::executable_available(source.executable());
    Ok(status)
}

#[tauri::command]
fn scan_source(
    state: State<'_, AppState>,
    source: SourceId,
) -> std::result::Result<SourceStatus, String> {
    let mut store = lock(&state).map_err(|e| e.to_string())?;
    let root = providers::load_configured_root(&store, source).map_err(|e| e.to_string())?;
    match providers::scan_source(&mut store, source, &root) {
        Ok(status) => Ok(status),
        Err(error) => {
            let mut status = store
                .sources()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|item| item.source == source)
                .unwrap_or_else(|| {
                    providers::initial_status(source, Some(root.to_string_lossy().into_owned()))
                });
            status.health = match error {
                AppError::Unsupported(_) => crate::domain::SourceHealth::Unsupported,
                AppError::Io(ref io) if io.kind() == std::io::ErrorKind::PermissionDenied => {
                    crate::domain::SourceHealth::PermissionDenied
                }
                _ => crate::domain::SourceHealth::Failed,
            };
            status.last_scan_unix = Some(now_unix());
            status.error = Some(error.to_string());
            store
                .save_source(&status)
                .map_err(|save_error| save_error.to_string())?;
            Err(error.to_string())
        }
    }
}

#[tauri::command]
fn get_sessions(
    state: State<'_, AppState>,
    source: Option<SourceId>,
) -> std::result::Result<Vec<SessionSummary>, String> {
    let store = lock(&state).map_err(|e| e.to_string())?;
    let sessions = store
        .sessions_with_candidates(now_unix())
        .map_err(|e| e.to_string())?;
    Ok(sessions
        .into_iter()
        .filter(|item| source.is_none_or(|source| item.source == source))
        .collect())
}

#[tauri::command]
fn get_cleanup_candidates(
    state: State<'_, AppState>,
    source: Option<SourceId>,
) -> std::result::Result<Vec<SessionSummary>, String> {
    let store = lock(&state).map_err(|e| e.to_string())?;
    let sessions = store
        .candidate_sessions(now_unix())
        .map_err(|e| e.to_string())?;
    Ok(sessions
        .into_iter()
        .filter(|item| source.is_none_or(|source| item.source == source))
        .collect())
}

#[tauri::command]
fn parse_session(
    state: State<'_, AppState>,
    id: String,
) -> std::result::Result<ParsedSession, String> {
    let store = lock(&state).map_err(|e| e.to_string())?;
    providers::parse_session(&store, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_activity(_state: State<'_, AppState>) -> ActivitySnapshot {
    ActivitySnapshot {
        records: Vec::new(),
        reliable: false,
        error: Some("活动进程检测尚未实现；清理会按安全规则阻止".into()),
    }
}

#[tauri::command]
fn preview_cleanup(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> std::result::Result<crate::domain::CleanupPreview, String> {
    let mut store = lock(&state).map_err(|e| e.to_string())?;
    let preview = providers::cleanup_preview(&store, &ids).map_err(|e| e.to_string())?;
    store
        .save_cleanup_preview(&preview)
        .map_err(|e| e.to_string())?;
    Ok(preview)
}

#[tauri::command]
fn execute_cleanup(
    state: State<'_, AppState>,
    token: String,
) -> std::result::Result<Vec<crate::domain::ItemOutcome>, String> {
    let store = lock(&state).map_err(|e| e.to_string())?;
    providers::execute_cleanup(&store, &token).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_sessions(
    state: State<'_, AppState>,
    ids: Vec<String>,
    format: ExportFormat,
    destination: String,
) -> std::result::Result<Vec<crate::domain::ItemOutcome>, String> {
    let store = lock(&state).map_err(|e| e.to_string())?;
    let destination = fs::canonicalize(destination).map_err(|e| e.to_string())?;
    if !destination.is_dir() {
        return Err("导出目标必须是目录".into());
    }
    let format = match format {
        ExportFormat::Markdown => "markdown",
        ExportFormat::Json => "json",
    };
    let mut outcomes = Vec::new();
    for id in ids {
        match providers::parse_session(&store, &id)
            .and_then(|session| providers::export_session(&store, &session, format, &destination))
        {
            Ok(()) => outcomes.push(crate::domain::ItemOutcome {
                id,
                success: true,
                error: None,
            }),
            Err(error) => outcomes.push(crate::domain::ItemOutcome {
                id,
                success: false,
                error: Some(error.to_string()),
            }),
        }
    }
    Ok(outcomes)
}

#[tauri::command]
fn preview_analysis(
    _source: SourceId,
    _instruction: String,
    _selections: Vec<crate::domain::MessageSelection>,
) -> std::result::Result<crate::domain::AnalysisPreview, String> {
    Err("CLI 分析的隔离参数尚未完成验证，当前不可用".into())
}

#[tauri::command]
fn run_analysis(
    _token: String,
    _approved_external: bool,
    _approved_writable: bool,
    _approved_persistent: bool,
) -> std::result::Result<AnalysisTask, String> {
    Err("CLI 分析的隔离参数尚未完成验证，当前不可用".into())
}

#[tauri::command]
fn get_tasks(state: State<'_, AppState>) -> std::result::Result<Vec<AnalysisTask>, String> {
    lock(&state)
        .map_err(|e| e.to_string())?
        .tasks()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(state: State<'_, AppState>, id: String) -> std::result::Result<(), String> {
    lock(&state)
        .map_err(|e| e.to_string())?
        .delete_task(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_index(state: State<'_, AppState>) -> std::result::Result<(), String> {
    lock(&state)
        .map_err(|e| e.to_string())?
        .clear_index()
        .map_err(|e| e.to_string())
}

fn database_path(
    app: &tauri::AppHandle,
) -> std::result::Result<PathBuf, Box<dyn std::error::Error>> {
    let directory = app.path().app_data_dir()?;
    fs::create_dir_all(&directory)?;
    Ok(directory.join("session-studio.sqlite3"))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = database_path(app.handle())?;
            app.manage(AppState(Mutex::new(Store::open(&path)?)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_sources,
            set_source_root,
            reset_source_root,
            scan_source,
            get_sessions,
            get_cleanup_candidates,
            parse_session,
            get_activity,
            preview_cleanup,
            execute_cleanup,
            export_sessions,
            preview_analysis,
            run_analysis,
            get_tasks,
            delete_task,
            clear_index,
        ])
        .run(tauri::generate_context!())
        .expect("Session Studio failed to start");
}
