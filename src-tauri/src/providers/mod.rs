pub mod claude_code;
pub mod codex;
pub mod deepseek_harness;
pub mod pi;
pub mod trash;

use crate::domain::{
    ActivityCertainty, AppError, CleanupContainer, CleanupPreview, ParsedSession, Result,
    SessionSummary, SourceHealth, SourceId, SourceStatus, now_unix,
};
use crate::storage::Store;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use uuid::Uuid;

pub const MAX_HEADER_BYTES: usize = 256 * 1024;
pub const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SESSION_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_JSONL_FILES: usize = 100_000;

#[derive(Debug, Clone)]
pub struct ProviderSpec {
    pub source: SourceId,
    pub extensions: &'static [&'static str],
    pub compressed: bool,
    pub validated: bool,
}

pub fn spec(source: SourceId) -> ProviderSpec {
    match source {
        SourceId::ClaudeCode => ProviderSpec {
            source,
            extensions: &["jsonl"],
            compressed: false,
            validated: false,
        },
        SourceId::Codex => ProviderSpec {
            source,
            extensions: &["jsonl", "jsonl.zst"],
            compressed: true,
            validated: true,
        },
        SourceId::DeepseekHarness => ProviderSpec {
            source,
            extensions: &["jsonl", "jsonl.zst"],
            compressed: true,
            validated: false,
        },
        SourceId::Pi => ProviderSpec {
            source,
            extensions: &["jsonl"],
            compressed: false,
            validated: true,
        },
    }
}

pub fn default_root(source: SourceId, home: &Path) -> Option<PathBuf> {
    default_root_with_env(source, home, &|key| std::env::var_os(key))
}

pub fn current_default_root(source: SourceId) -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    default_root(source, &home)
}

fn default_root_with_env(
    source: SourceId,
    home: &Path,
    get_env: &impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    match source {
        SourceId::ClaudeCode => Some(
            get_env("CLAUDE_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".claude"))
                .join("projects"),
        ),
        SourceId::Codex => Some(
            get_env("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex")),
        ),
        SourceId::DeepseekHarness => Some(
            get_env("DSH_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".dsh"))
                .join("sessions"),
        ),
        SourceId::Pi => Some(
            get_env("PI_CODING_AGENT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".pi/agent"))
                .join("sessions"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_status_displays_default_root_without_scanning() {
        let status = initial_status(SourceId::Codex, None);
        assert_eq!(status.default_root, status.root);
        assert!(status.uses_default_root);
        assert_eq!(status.health, SourceHealth::NotScanned);
        assert!(status.last_scan_unix.is_none());
    }

    #[test]
    fn documented_environment_overrides_resolve_each_source_root() {
        let home = Path::new("/Users/tester");
        let overrides = [
            ("CLAUDE_CONFIG_DIR", "/custom/claude"),
            ("CODEX_HOME", "/custom/codex"),
            ("DSH_HOME", "/custom/dsh"),
            ("PI_CODING_AGENT_DIR", "/custom/pi"),
        ];
        let get_env = |key: &str| {
            overrides
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| std::ffi::OsString::from(value))
        };

        assert_eq!(
            default_root_with_env(SourceId::ClaudeCode, home, &get_env),
            Some(Path::new("/custom/claude/projects").to_path_buf())
        );
        assert_eq!(
            default_root_with_env(SourceId::Codex, home, &get_env),
            Some(Path::new("/custom/codex").to_path_buf())
        );
        assert_eq!(
            default_root_with_env(SourceId::DeepseekHarness, home, &get_env),
            Some(Path::new("/custom/dsh/sessions").to_path_buf())
        );
        assert_eq!(
            default_root_with_env(SourceId::Pi, home, &get_env),
            Some(Path::new("/custom/pi/sessions").to_path_buf())
        );
    }
}

pub fn initial_status(source: SourceId, root: Option<String>) -> SourceStatus {
    let default_root = current_default_root(source).map(|path| path.to_string_lossy().into_owned());
    let uses_default_root = root.is_none();
    let root = root.or_else(|| default_root.clone());
    SourceStatus {
        source,
        default_root,
        root,
        uses_default_root,
        health: SourceHealth::NotScanned,
        last_scan_unix: None,
        session_count: 0,
        error: None,
        cli_available: executable_available(source.executable()),
    }
}

pub fn executable_available(name: &str) -> bool {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|dir| dir.join(name).is_file())
}

pub fn configure_root(store: &mut Store, source: SourceId, root: &Path) -> Result<SourceStatus> {
    let canonical = fs::canonicalize(root)?;
    if !canonical.is_dir() {
        return Err(AppError::Invalid("来源路径必须是目录".into()));
    }
    if source == SourceId::DeepseekHarness && canonical.as_os_str().is_empty() {
        return Err(AppError::Invalid(
            "DeepSeek Harness 必须明确选择 persistence root".into(),
        ));
    }
    let root = canonical.to_string_lossy().into_owned();
    store.set_custom_root(source, &root)?;
    let mut status = initial_status(source, Some(root));
    status.uses_default_root = false;
    Ok(status)
}

pub fn scan_source(store: &mut Store, source: SourceId, root: &Path) -> Result<SourceStatus> {
    let mut canonical = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut status = initial_status(source, Some(root.to_string_lossy().into_owned()));
            status.uses_default_root = store.custom_root(source)?.is_none();
            status.health = SourceHealth::Missing;
            status.last_scan_unix = Some(now_unix());
            status.error = Some(format!("来源目录不存在：{}", root.display()));
            store.save_source(&status)?;
            return Ok(status);
        }
        Err(error) => return Err(error.into()),
    };
    let mut root_identity = root.to_path_buf();
    if !canonical.is_dir() {
        return Err(AppError::Invalid("来源路径必须是目录".into()));
    }
    if source == SourceId::Codex
        && canonical.file_name().and_then(|name| name.to_str()) == Some("sessions")
    {
        canonical = canonical
            .parent()
            .ok_or_else(|| AppError::Invalid("Codex sessions root has no parent".into()))?
            .to_path_buf();
        root_identity = root_identity
            .parent()
            .ok_or_else(|| AppError::Invalid("Codex sessions root has no parent".into()))?
            .to_path_buf();
    }
    if source == SourceId::DeepseekHarness {
        let mut status = initial_status(source, Some(root_identity.to_string_lossy().into_owned()));
        status.uses_default_root = store.custom_root(source)?.is_none();
        status.health = SourceHealth::NeedsValidation;
        status.error = Some("DeepSeek Harness 当前版本的 session 文件格式尚未通过实现验证".into());
        status.last_scan_unix = Some(now_unix());
        store.save_source(&status)?;
        return Ok(status);
    }
    let mut status = SourceStatus {
        source,
        root: Some(root_identity.to_string_lossy().into_owned()),
        default_root: current_default_root(source).map(|path| path.to_string_lossy().into_owned()),
        uses_default_root: store.custom_root(source)?.is_none(),
        health: SourceHealth::Scanning,
        last_scan_unix: Some(now_unix()),
        session_count: 0,
        error: None,
        cli_available: executable_available(source.executable()),
    };
    if !spec(source).validated {
        status.health = SourceHealth::NeedsValidation;
        status.error = Some(format!("{} 会话格式尚未验证", source.label()));
        store.save_source(&status)?;
        return Ok(status);
    }
    let scan_roots: Vec<PathBuf> = if source == SourceId::Codex {
        ["sessions", "archived_sessions"]
            .into_iter()
            .map(|name| canonical.join(name))
            .filter(|path| path.is_dir())
            .collect()
    } else {
        vec![canonical.clone()]
    };
    let mut entries = Vec::new();
    for scan_root in &scan_roots {
        entries.extend(discover_files(scan_root, &spec(source))?);
    }
    let mut sessions = Vec::with_capacity(entries.len());
    for path in entries {
        match parse_metadata(source, &root_identity, &path) {
            Ok(Some(summary)) => sessions.push(summary),
            Ok(None) => {}
            Err(error) => {
                status.health = match error {
                    AppError::Unsupported(_) => SourceHealth::Unsupported,
                    AppError::Io(ref io) if io.kind() == std::io::ErrorKind::PermissionDenied => {
                        SourceHealth::PermissionDenied
                    }
                    _ => SourceHealth::Failed,
                };
                status.error = Some(format!("{}: {}", path.display(), error));
                store.save_source(&status)?;
                return Err(error);
            }
        }
    }
    status.session_count = sessions.len();
    status.health = if sessions.is_empty() {
        SourceHealth::Empty
    } else {
        SourceHealth::Ready
    };
    store.complete_scan(&status, &sessions)?;
    Ok(status)
}

pub fn discover_files(root: &Path, spec: &ProviderSpec) -> Result<Vec<PathBuf>> {
    if !spec.validated {
        return Err(AppError::Unsupported(format!(
            "{} 格式未验证，禁止扫描",
            spec.source.label()
        )));
    }
    let mut paths = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    let mut visited = HashSet::new();
    while let Some(dir) = pending.pop() {
        let canonical = match fs::canonicalize(&dir) {
            Ok(path) => path,
            Err(_) => continue,
        };
        if !canonical.starts_with(root) || !visited.insert(canonical.clone()) {
            continue;
        }
        for entry in fs::read_dir(&canonical)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file()
                && spec
                    .extensions
                    .iter()
                    .any(|ext| path.to_string_lossy().ends_with(ext))
            {
                paths.push(path);
                if paths.len() > MAX_JSONL_FILES {
                    return Err(AppError::Unsupported("来源文件数量超过安全上限".into()));
                }
            }
        }
    }
    paths.sort();
    Ok(paths)
}

fn open_jsonl_reader(path: &Path) -> Result<Box<dyn BufRead>> {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .ends_with(".zst")
    {
        let decoder = zstd::stream::read::Decoder::new(File::open(path)?)?;
        return Ok(Box::new(BufReader::new(
            decoder.take(MAX_SESSION_BYTES as u64 + 1),
        )));
    }
    Ok(Box::new(BufReader::new(File::open(path)?)))
}

fn read_bounded_line(reader: &mut dyn BufRead, line: &mut Vec<u8>, limit: usize) -> Result<usize> {
    line.clear();
    let mut total = 0;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }
        let count = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|position| position + 1)
            .unwrap_or(available.len());
        if total + count > limit {
            return Err(AppError::Unsupported(
                "JSONL line exceeds safety limit".into(),
            ));
        }
        let has_newline = available.get(count.wrapping_sub(1)) == Some(&b'\n');
        line.extend_from_slice(&available[..count]);
        total += count;
        reader.consume(count);
        if has_newline {
            break;
        }
    }
    Ok(total)
}

fn parse_metadata(source: SourceId, root: &Path, path: &Path) -> Result<Option<SessionSummary>> {
    let metadata = fs::metadata(path)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|time| time.as_secs() as i64)
        .unwrap_or(0);
    let mut reader = open_jsonl_reader(path)?;
    let mut first = Vec::new();
    let mut second = Vec::new();
    read_bounded_line(&mut reader, &mut first, MAX_LINE_BYTES)?;
    read_bounded_line(&mut reader, &mut second, MAX_LINE_BYTES)?;
    if first.len() + second.len() > MAX_HEADER_BYTES {
        return Err(AppError::Unsupported("会话头部超过安全上限".into()));
    }
    let first_value = parse_json_line(&first)?;
    if source == SourceId::Pi && first_value.get("type").and_then(Value::as_str) != Some("session")
    {
        return Err(AppError::Unsupported("Pi 会话 header 不匹配".into()));
    }
    let (session_id, project_path, title) = match source {
        SourceId::Pi => (
            first_value
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            first_value
                .get("cwd")
                .and_then(Value::as_str)
                .map(str::to_owned),
            first_value
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ),
        SourceId::Codex => {
            let record = if first_value.get("type").and_then(Value::as_str) == Some("session_meta")
            {
                first_value
            } else {
                parse_json_line(&second)?
            };
            let payload = record.get("payload").unwrap_or(&record);
            (
                payload
                    .get("id")
                    .or_else(|| payload.get("thread_id"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                payload
                    .get("cwd")
                    .or_else(|| payload.get("working_dir"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                payload
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            )
        }
        _ => return Err(AppError::Unsupported("来源解析器未实现".into())),
    };
    let session_id = session_id.ok_or_else(|| AppError::Unsupported("会话头缺少稳定 ID".into()))?;
    let mut hasher = Sha256::new();
    hasher.update(source_key(source).as_bytes());
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    let stable_id = format!("{}:{}", source_key(source), hex(&hasher.finalize()));
    let mut identity = Sha256::new();
    identity.update(path.as_os_str().to_string_lossy().as_bytes());
    let container_id = hex(&identity.finalize());
    Ok(Some(SessionSummary {
        id: stable_id,
        source,
        source_session_id: session_id,
        source_root: root.to_string_lossy().into_owned(),
        container_path: path.to_string_lossy().into_owned(),
        container_id,
        title,
        project_path,
        message_count: None,
        last_activity_unix: None,
        modified_at_unix: modified,
        fingerprint: format!("{}:{}", metadata.len(), modified),
        candidate_reasons: Vec::new(),
    }))
}

fn parse_json_line(line: &[u8]) -> Result<Value> {
    if line.len() > MAX_LINE_BYTES {
        return Err(AppError::Unsupported("JSONL 行超过安全上限".into()));
    }
    if line.iter().all(u8::is_ascii_whitespace) {
        return Ok(Value::Null);
    }
    Ok(serde_json::from_slice(line)?)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn source_key(source: SourceId) -> &'static str {
    match source {
        SourceId::ClaudeCode => "claude_code",
        SourceId::Codex => "codex",
        SourceId::DeepseekHarness => "deepseek_harness",
        SourceId::Pi => "pi",
    }
}

pub fn parse_session(store: &Store, id: &str) -> Result<ParsedSession> {
    let summary = store
        .session(id)?
        .ok_or_else(|| AppError::Invalid("会话不存在于本地索引".into()))?;
    match summary.source {
        SourceId::Pi => pi::parse(&summary),
        SourceId::Codex => codex::parse(&summary),
        SourceId::ClaudeCode => claude_code::parse(&summary),
        SourceId::DeepseekHarness => deepseek_harness::parse(&summary),
    }
}

pub fn cleanup_preview(store: &Store, ids: &[String]) -> Result<CleanupPreview> {
    let mut unique = std::collections::BTreeMap::<String, Vec<String>>::new();
    for id in ids {
        let session = store
            .session(id)?
            .ok_or_else(|| AppError::Invalid(format!("未知会话：{id}")))?;
        unique.entry(session.container_path).or_default();
    }
    let all_sessions = store.sessions()?;
    let containers = unique
        .into_iter()
        .map(|(path, _)| {
            let session_ids: Vec<String> = all_sessions
                .iter()
                .filter(|session| session.container_path == path)
                .map(|session| session.id.clone())
                .collect();
            let (activity, blocked_reason) = store
                .cleanup_safety(&path)
                .unwrap_or((ActivityCertainty::Unknown, Some("无法确认活动关联".into())));
            CleanupContainer {
                path,
                session_ids,
                activity,
                blocked_reason,
            }
        })
        .collect();
    let preview = CleanupPreview {
        token: Uuid::new_v4().to_string(),
        containers,
    };
    Ok(preview)
}

pub fn execute_cleanup(store: &Store, token: &str) -> Result<Vec<crate::domain::ItemOutcome>> {
    execute_cleanup_with(store, token, trash::move_to_trash)
}

pub fn execute_cleanup_with<F>(
    store: &Store,
    token: &str,
    mut move_to_trash: F,
) -> Result<Vec<crate::domain::ItemOutcome>>
where
    F: FnMut(&Path) -> Result<()>,
{
    let preview = store
        .cleanup_preview(token)?
        .ok_or_else(|| AppError::Invalid("清理预览已失效，请重新预览".into()))?;
    let mut outcomes = Vec::new();
    for container in preview.containers {
        let (activity, reason) = store.cleanup_safety(&container.path).unwrap_or((
            ActivityCertainty::Unknown,
            Some("无法重新确认活动状态".into()),
        ));
        if !activity.allows_cleanup() {
            outcomes.push(crate::domain::ItemOutcome {
                id: container.path,
                success: false,
                error: reason.or(container.blocked_reason),
            });
            continue;
        }
        let canonical = match fs::canonicalize(&container.path) {
            Ok(path) if path.is_file() || path.is_dir() => path,
            Ok(_) => {
                outcomes.push(crate::domain::ItemOutcome {
                    id: container.path,
                    success: false,
                    error: Some("不支持的容器类型".into()),
                });
                continue;
            }
            Err(error) => {
                outcomes.push(crate::domain::ItemOutcome {
                    id: container.path,
                    success: false,
                    error: Some(error.to_string()),
                });
                continue;
            }
        };
        let sessions = store.sessions()?;
        let Some(source_root) = sessions
            .iter()
            .find(|session| session.container_path == container.path)
            .map(|session| session.source_root.as_str())
        else {
            outcomes.push(crate::domain::ItemOutcome {
                id: container.path,
                success: false,
                error: Some("容器已不在本地索引中".into()),
            });
            continue;
        };
        let canonical_root = match fs::canonicalize(source_root) {
            Ok(root) => root,
            Err(error) => {
                outcomes.push(crate::domain::ItemOutcome {
                    id: container.path,
                    success: false,
                    error: Some(format!("授权来源根目录不可用：{error}")),
                });
                continue;
            }
        };
        if !canonical.starts_with(&canonical_root) || canonical == canonical_root {
            outcomes.push(crate::domain::ItemOutcome {
                id: container.path,
                success: false,
                error: Some("容器不在授权来源根目录之内".into()),
            });
            continue;
        }
        match move_to_trash(&canonical) {
            Ok(()) => outcomes.push(crate::domain::ItemOutcome {
                id: container.path,
                success: true,
                error: None,
            }),
            Err(error) => outcomes.push(crate::domain::ItemOutcome {
                id: container.path,
                success: false,
                error: Some(error.to_string()),
            }),
        }
    }
    Ok(outcomes)
}

pub fn messages_markdown(session: &ParsedSession) -> String {
    let mut output = format!(
        "# {}\n\n- Source: {}\n- Session ID: {}\n- Project: {}\n\n",
        session
            .summary
            .title
            .as_deref()
            .unwrap_or("Untitled session"),
        session.summary.source.label(),
        session.summary.source_session_id,
        session.summary.project_path.as_deref().unwrap_or("Unknown")
    );
    for message in &session.messages {
        output.push_str(&format!(
            "## {}{}\n\n{}\n\n",
            message.role,
            message
                .timestamp
                .as_deref()
                .map(|t| format!(" · {t}"))
                .unwrap_or_default(),
            message
                .content
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| message.content.to_string())
        ));
        for attachment in &message.attachments {
            output.push_str(&format!(
                "Attachment ({}): {}\n\n",
                attachment.kind, attachment.reference
            ));
        }
    }
    output
}

pub fn export_session(
    store: &Store,
    session: &ParsedSession,
    format: &str,
    destination: &Path,
) -> Result<()> {
    let content = match format {
        "markdown" => messages_markdown(session),
        "json" => serde_json::to_string_pretty(session)?,
        _ => return Err(AppError::Invalid("不支持的导出格式".into())),
    };
    let name = session
        .summary
        .source_session_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect::<String>();
    if name.is_empty() {
        return Err(AppError::Invalid("会话 ID 不适合用作文件名".into()));
    }
    let extension = if format == "markdown" { "md" } else { "json" };
    let path = destination.join(format!("{name}.{extension}"));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    use std::io::Write;
    file.write_all(content.as_bytes())?;
    let _ = store;
    Ok(())
}

pub fn load_configured_root(store: &Store, source: SourceId) -> Result<PathBuf> {
    store
        .custom_root(source)?
        .map(PathBuf::from)
        .or_else(|| current_default_root(source))
        .ok_or_else(|| AppError::Invalid(format!("{} 默认目录无法解析", source.label())))
}

pub fn list_header_lines(path: &Path) -> Result<Vec<Vec<u8>>> {
    let mut file = File::open(path)?;
    let mut reader = BufReader::new(&mut file);
    let mut lines = Vec::new();
    for _ in 0..2 {
        let mut line = Vec::new();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        if line.len() > MAX_HEADER_BYTES {
            return Err(AppError::Unsupported("会话头部过大".into()));
        }
        lines.push(line);
    }
    Ok(lines)
}

pub fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(AppError::Unsupported("读取内容超过安全上限".into()));
    }
    Ok(bytes)
}
