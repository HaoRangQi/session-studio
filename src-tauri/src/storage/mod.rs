use crate::domain::{
    ActivityCertainty, ActivityRecord, AnalysisTask, AppError, CandidateReason, CleanupContainer,
    CleanupPreview, Result, SessionSummary, SourceHealth, SourceId, SourceStatus, now_unix,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

const SESSION_COLUMNS: &str = "id, source, container_path, metadata";

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(std::time::Duration::from_secs(3))?;
        let previous_version: i64 =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                container_path TEXT NOT NULL,
                metadata TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS sessions_source ON sessions(source);
            CREATE TABLE IF NOT EXISTS sources (
                source TEXT PRIMARY KEY,
                root TEXT,
                health TEXT NOT NULL,
                last_scan_unix INTEGER,
                session_count INTEGER NOT NULL,
                error TEXT,
                cli_available INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS custom_roots (
                source TEXT PRIMARY KEY,
                root TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS analysis_tasks (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at_unix INTEGER NOT NULL,
                content TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cleanup_previews (
                token TEXT PRIMARY KEY,
                created_at_unix INTEGER NOT NULL,
                content TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS activity_snapshots (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                reliable INTEGER NOT NULL,
                content TEXT NOT NULL,
                error TEXT
            );",
        )?;
        if previous_version == 0 {
            connection.execute(
                "INSERT OR IGNORE INTO custom_roots(source, root)
                 SELECT source, root FROM sources WHERE root IS NOT NULL",
                [],
            )?;
            connection.pragma_update(None, "user_version", 1)?;
        }
        Ok(Self { connection })
    }

    pub fn complete_scan(
        &mut self,
        status: &SourceStatus,
        sessions: &[SessionSummary],
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        for session in sessions {
            if session.source != status.source
                || session.source_root != status.root.as_deref().unwrap_or("")
            {
                return Err(AppError::Invalid("扫描结果与来源状态不匹配".into()));
            }
            let metadata = serde_json::to_string(session)?;
            tx.execute(
                "INSERT INTO sessions(id, source, container_path, metadata) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET source=excluded.source, container_path=excluded.container_path, metadata=excluded.metadata",
                params![session.id, source_key(session.source), session.container_path, metadata],
            )?;
        }
        if let Some(root) = &status.root {
            let seen_ids: std::collections::BTreeSet<_> =
                sessions.iter().map(|session| session.id.as_str()).collect();
            let stale_ids: Vec<String> = {
                let mut statement =
                    tx.prepare("SELECT id, metadata FROM sessions WHERE source=?1")?;
                let rows = statement.query_map([source_key(status.source)], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                let mut ids = Vec::new();
                for row in rows {
                    let (id, metadata) = row?;
                    let session: SessionSummary = serde_json::from_str(&metadata)?;
                    if session.source_root != *root || !seen_ids.contains(id.as_str()) {
                        ids.push(id);
                    }
                }
                ids
            };
            for id in stale_ids {
                tx.execute("DELETE FROM sessions WHERE id=?1", [id])?;
            }
        }
        if let Some(root) = &status.root {
            if status.uses_default_root {
                tx.execute(
                    "DELETE FROM custom_roots WHERE source=?1",
                    [source_key(status.source)],
                )?;
            } else {
                tx.execute(
                    "INSERT INTO custom_roots(source, root) VALUES (?1, ?2) ON CONFLICT(source) DO UPDATE SET root=excluded.root",
                    params![source_key(status.source), root],
                )?;
            }
        }
        save_source_on(&tx, status)?;
        tx.commit()?;
        Ok(())
    }

    pub fn save_source(&mut self, status: &SourceStatus) -> Result<()> {
        save_source_on(&self.connection, status)
    }

    pub fn sessions(&self) -> Result<Vec<SessionSummary>> {
        let mut statement = self.connection.prepare(&format!("SELECT {SESSION_COLUMNS} FROM sessions ORDER BY CAST(json_extract(metadata, '$.modified_at_unix') AS INTEGER) DESC"))?;
        let rows = statement.query_map([], |row| row.get::<_, String>(3))?;
        let sessions: Vec<SessionSummary> = rows
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect::<Result<_>>()?;
        sessions
            .into_iter()
            .map(|session| {
                self.session_root_is_current(&session)
                    .map(|is_current| (session, is_current))
            })
            .filter_map(|result| match result {
                Ok((session, true)) => Some(Ok(session)),
                Ok((_, false)) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }

    pub fn session(&self, id: &str) -> Result<Option<SessionSummary>> {
        let content: Option<String> = self
            .connection
            .query_row("SELECT metadata FROM sessions WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()?;
        let Some(content) = content else {
            return Ok(None);
        };
        let session: SessionSummary = serde_json::from_str(&content)?;
        if self.session_root_is_current(&session)? {
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    fn session_root_is_current(&self, session: &SessionSummary) -> Result<bool> {
        let root = self.custom_root(session.source)?.or_else(|| {
            crate::providers::current_default_root(session.source)
                .map(|path| path.to_string_lossy().into_owned())
        });
        Ok(root.as_deref() == Some(session.source_root.as_str()))
    }

    pub fn custom_root(&self, source: SourceId) -> Result<Option<String>> {
        Ok(self
            .connection
            .query_row(
                "SELECT root FROM custom_roots WHERE source=?1",
                [source_key(source)],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn set_custom_root(&mut self, source: SourceId, root: &str) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO custom_roots(source, root) VALUES (?1, ?2) ON CONFLICT(source) DO UPDATE SET root=excluded.root",
            params![source_key(source), root],
        )?;
        tx.execute("DELETE FROM sessions WHERE source=?1", [source_key(source)])?;
        tx.execute("DELETE FROM sources WHERE source=?1", [source_key(source)])?;
        tx.commit()?;
        Ok(())
    }

    pub fn reset_source_root(&mut self, source: SourceId) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute(
            "DELETE FROM custom_roots WHERE source=?1",
            [source_key(source)],
        )?;
        tx.execute("DELETE FROM sessions WHERE source=?1", [source_key(source)])?;
        tx.execute("DELETE FROM sources WHERE source=?1", [source_key(source)])?;
        tx.commit()?;
        Ok(())
    }

    pub fn clear_custom_root(&mut self, source: SourceId) -> Result<()> {
        self.reset_source_root(source)
    }

    pub fn sources(&self) -> Result<Vec<SourceStatus>> {
        let mut statement = self.connection.prepare("SELECT source, root, health, last_scan_unix, session_count, error, cli_available FROM sources ORDER BY source")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, usize>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, bool>(6)?,
            ))
        })?;
        rows.map(|row| {
            let (source, stored_root, health, last_scan_unix, session_count, error, cli_available) =
                row?;
            let source = parse_source(&source)?;
            let custom_root = self.custom_root(source)?;
            let default_root = crate::providers::current_default_root(source)
                .map(|path| path.to_string_lossy().into_owned());
            let uses_default_root = custom_root.is_none();
            let root = custom_root
                .or_else(|| default_root.clone())
                .or_else(|| stored_root.clone());
            let status_matches_root = stored_root.as_deref() == root.as_deref();
            Ok(SourceStatus {
                source,
                root,
                default_root,
                uses_default_root,
                health: if status_matches_root {
                    parse_health(&health)?
                } else {
                    SourceHealth::NotScanned
                },
                last_scan_unix: if status_matches_root {
                    last_scan_unix
                } else {
                    None
                },
                session_count: if status_matches_root {
                    session_count
                } else {
                    0
                },
                error: if status_matches_root { error } else { None },
                cli_available,
            })
        })
        .collect()
    }

    pub fn save_task(&mut self, task: &AnalysisTask) -> Result<()> {
        self.connection.execute("INSERT INTO analysis_tasks(id, source, status, created_at_unix, content) VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET status=excluded.status, content=excluded.content",
            params![task.id, source_key(task.source), task.status, task.created_at_unix, serde_json::to_string(task)?])?;
        Ok(())
    }

    pub fn tasks(&self) -> Result<Vec<AnalysisTask>> {
        let mut statement = self
            .connection
            .prepare("SELECT content FROM analysis_tasks ORDER BY created_at_unix DESC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    pub fn delete_task(&mut self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM analysis_tasks WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn clear_index(&mut self) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM sessions", [])?;
        tx.execute(
            "UPDATE sources SET health=?1, last_scan_unix=NULL, session_count=0, error=NULL",
            [health_key(SourceHealth::NotScanned)],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn save_cleanup_preview(&mut self, preview: &CleanupPreview) -> Result<()> {
        self.connection.execute("INSERT OR REPLACE INTO cleanup_previews(token, created_at_unix, content) VALUES (?1, ?2, ?3)",
            params![preview.token, now_unix(), serde_json::to_string(preview)?])?;
        Ok(())
    }

    pub fn cleanup_preview(&self, token: &str) -> Result<Option<CleanupPreview>> {
        let content: Option<String> = self
            .connection
            .query_row(
                "SELECT content FROM cleanup_previews WHERE token=?1",
                [token],
                |row| row.get(0),
            )
            .optional()?;
        content
            .map(|content| serde_json::from_str(&content).map_err(AppError::from))
            .transpose()
    }

    pub fn save_activity(
        &mut self,
        reliable: bool,
        records: &[ActivityRecord],
        error: Option<&str>,
    ) -> Result<()> {
        self.connection.execute("INSERT OR REPLACE INTO activity_snapshots(id, reliable, content, error) VALUES(1, ?1, ?2, ?3)",
            params![reliable, serde_json::to_string(records)?, error])?;
        Ok(())
    }

    pub fn activity(&self) -> Result<(bool, Vec<ActivityRecord>, Option<String>)> {
        let snapshot = self
            .connection
            .query_row(
                "SELECT reliable, content, error FROM activity_snapshots WHERE id=1",
                [],
                |row| {
                    Ok((
                        row.get::<_, bool>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;
        match snapshot {
            Some((reliable, content, error)) => {
                Ok((reliable, serde_json::from_str(&content)?, error))
            }
            None => Ok((false, Vec::new(), Some("活动状态尚未扫描".into()))),
        }
    }

    pub fn sessions_with_candidates(&self, now: i64) -> Result<Vec<SessionSummary>> {
        self.sessions()?
            .into_iter()
            .map(|mut session| {
                let exists = session.project_path.as_deref().and_then(|path| {
                    match Path::new(path).try_exists() {
                        Ok(exists) => Some(exists),
                        Err(_) => None,
                    }
                });
                session.candidate_reasons = crate::domain::candidate_reasons(
                    session.project_path.as_deref(),
                    exists,
                    session.last_activity_unix,
                    now,
                );
                Ok(session)
            })
            .collect()
    }

    pub fn candidate_sessions(&self, now: i64) -> Result<Vec<SessionSummary>> {
        Ok(self
            .sessions_with_candidates(now)?
            .into_iter()
            .filter(|session| !session.candidate_reasons.is_empty())
            .collect())
    }

    pub fn cleanup_safety(
        &self,
        container_path: &str,
    ) -> Result<(ActivityCertainty, Option<String>)> {
        let (reliable, records, _) = self.activity()?;
        if !reliable {
            return Ok((
                ActivityCertainty::Unknown,
                Some("无法可靠确认活动任务关联".into()),
            ));
        }
        let sessions = self.sessions()?;
        let relevant: Vec<_> = sessions
            .iter()
            .filter(|s| s.container_path == container_path)
            .collect();
        if relevant.is_empty() {
            return Ok((
                ActivityCertainty::Unknown,
                Some("容器未关联到已索引会话".into()),
            ));
        }
        for record in records.iter().filter(|record| record.state == "running") {
            let Some(cwd) = record.project_path.as_deref() else {
                return Ok((
                    ActivityCertainty::Unknown,
                    Some(format!("活动进程 {} 的工作目录未知", record.pid)),
                ));
            };
            if record.source == relevant[0].source {
                return Ok((
                    ActivityCertainty::Unknown,
                    Some(format!("同来源活动进程 {} 尚无会话 ID 关联", record.pid)),
                ));
            }
            let cwd = Path::new(cwd);
            if relevant.iter().any(|session| {
                session.project_path.as_deref().is_some_and(|project| {
                    let project = Path::new(project);
                    cwd == project || cwd.starts_with(project)
                })
            }) {
                return Ok((
                    ActivityCertainty::Running,
                    Some(format!("活动进程 {} 与项目目录关联", record.pid)),
                ));
            }
        }
        if records.iter().any(|record| record.state == "running")
            && relevant
                .iter()
                .any(|session| session.project_path.is_none())
        {
            return Ok((
                ActivityCertainty::Unknown,
                Some("存在活动任务，但会话未关联项目目录".into()),
            ));
        }
        Ok((ActivityCertainty::Inactive, None))
    }
}

fn save_source_on(connection: &Connection, status: &SourceStatus) -> Result<()> {
    connection.execute("INSERT INTO sources(source, root, health, last_scan_unix, session_count, error, cli_available) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(source) DO UPDATE SET root=excluded.root, health=excluded.health, last_scan_unix=excluded.last_scan_unix, session_count=excluded.session_count, error=excluded.error, cli_available=excluded.cli_available",
        params![source_key(status.source), status.root, health_key(status.health), status.last_scan_unix,
            status.session_count, status.error, status.cli_available])?;
    Ok(())
}

fn source_key(source: SourceId) -> &'static str {
    match source {
        SourceId::ClaudeCode => "claude_code",
        SourceId::Codex => "codex",
        SourceId::DeepseekHarness => "deepseek_harness",
        SourceId::Pi => "pi",
    }
}
fn parse_source(value: &str) -> Result<SourceId> {
    match value {
        "claude_code" => Ok(SourceId::ClaudeCode),
        "codex" => Ok(SourceId::Codex),
        "deepseek_harness" => Ok(SourceId::DeepseekHarness),
        "pi" => Ok(SourceId::Pi),
        _ => Err(AppError::Invalid(format!("未知来源：{value}"))),
    }
}
fn health_key(health: SourceHealth) -> &'static str {
    match health {
        SourceHealth::NotScanned => "not_scanned",
        SourceHealth::NeedsValidation => "needs_validation",
        SourceHealth::Scanning => "scanning",
        SourceHealth::Ready => "ready",
        SourceHealth::Empty => "empty",
        SourceHealth::Missing => "missing",
        SourceHealth::PermissionDenied => "permission_denied",
        SourceHealth::Unsupported => "unsupported",
        SourceHealth::Failed => "failed",
    }
}
fn parse_health(value: &str) -> Result<SourceHealth> {
    match value {
        "not_scanned" | "needs_authorization" => Ok(SourceHealth::NotScanned),
        "needs_validation" => Ok(SourceHealth::NeedsValidation),
        "scanning" => Ok(SourceHealth::Scanning),
        "ready" => Ok(SourceHealth::Ready),
        "empty" => Ok(SourceHealth::Empty),
        "missing" => Ok(SourceHealth::Missing),
        "permission_denied" => Ok(SourceHealth::PermissionDenied),
        "unsupported" => Ok(SourceHealth::Unsupported),
        "failed" => Ok(SourceHealth::Failed),
        _ => Err(AppError::Invalid(format!("未知来源状态：{value}"))),
    }
}

#[allow(dead_code)]
fn _candidate_reason_count(reasons: &[CandidateReason]) -> usize {
    reasons.len()
}
#[allow(dead_code)]
fn _cleanup_container_path(container: &CleanupContainer) -> &str {
    &container.path
}
