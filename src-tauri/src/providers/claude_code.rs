use crate::domain::{AppError, ParsedSession, Result, SessionSummary};

pub fn parse(_summary: &SessionSummary) -> Result<ParsedSession> {
    Err(AppError::Unsupported(
        "Claude Code session format is not enabled until a redacted sample is verified".into(),
    ))
}
