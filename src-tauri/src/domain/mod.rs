use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceId {
    ClaudeCode,
    Codex,
    DeepseekHarness,
    Pi,
}

impl SourceId {
    pub const ALL: [Self; 4] = [
        Self::ClaudeCode,
        Self::Codex,
        Self::DeepseekHarness,
        Self::Pi,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::DeepseekHarness => "DeepSeek Harness",
            Self::Pi => "Pi",
        }
    }

    pub fn executable(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude",
            Self::Codex => "codex",
            Self::DeepseekHarness => "dsh",
            Self::Pi => "pi",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceHealth {
    NotScanned,
    NeedsValidation,
    Scanning,
    Ready,
    Empty,
    Missing,
    PermissionDenied,
    Unsupported,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatus {
    pub source: SourceId,
    pub root: Option<String>,
    pub default_root: Option<String>,
    pub uses_default_root: bool,
    pub health: SourceHealth,
    pub last_scan_unix: Option<i64>,
    pub session_count: usize,
    pub error: Option<String>,
    pub cli_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateReason {
    MissingProject,
    Unassociated,
    Inactive90Days,
}

pub fn candidate_reasons(
    project: Option<&str>,
    project_exists: Option<bool>,
    last_activity: Option<i64>,
    now: i64,
) -> Vec<CandidateReason> {
    let mut reasons = Vec::new();
    if project.is_none() {
        reasons.push(CandidateReason::Unassociated);
    } else if project_exists == Some(false) {
        reasons.push(CandidateReason::MissingProject);
    }
    if last_activity.is_some_and(|last| now.saturating_sub(last) >= 90 * 86_400) {
        reasons.push(CandidateReason::Inactive90Days);
    }
    reasons
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub source: SourceId,
    pub source_session_id: String,
    pub source_root: String,
    pub container_path: String,
    pub container_id: String,
    pub title: Option<String>,
    pub project_path: Option<String>,
    pub message_count: Option<usize>,
    pub last_activity_unix: Option<i64>,
    pub modified_at_unix: i64,
    pub fingerprint: String,
    pub candidate_reasons: Vec<CandidateReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityCertainty {
    Running,
    Inactive,
    Unknown,
}

impl ActivityCertainty {
    pub fn allows_cleanup(self) -> bool {
        self == Self::Inactive
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub pid: u32,
    pub source: SourceId,
    pub project_path: Option<String>,
    pub started_at_unix: Option<i64>,
    pub state: String,
    pub child_pids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySnapshot {
    pub records: Vec<ActivityRecord>,
    pub reliable: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentRef {
    pub kind: String,
    pub reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub timestamp: Option<String>,
    pub content: serde_json::Value,
    pub attachments: Vec<AttachmentRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSession {
    pub summary: SessionSummary,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupContainer {
    pub path: String,
    pub session_ids: Vec<String>,
    pub activity: ActivityCertainty,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPreview {
    pub token: String,
    pub containers: Vec<CleanupContainer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemOutcome {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Markdown,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSelection {
    pub session_id: String,
    pub start: usize,
    pub end: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPreview {
    pub token: String,
    pub source: SourceId,
    pub input: String,
    pub executable: String,
    pub arguments: Vec<String>,
    pub backend: String,
    pub read_only_guaranteed: bool,
    pub ephemeral: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisTask {
    pub id: String,
    pub source: SourceId,
    pub status: String,
    pub created_at_unix: i64,
    pub input: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("文件访问失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("本地数据库操作失败：{0}")]
    Database(#[from] rusqlite::Error),
    #[error("数据格式错误：{0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
