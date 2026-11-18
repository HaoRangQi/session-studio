use crate::domain::{AppError, ParsedSession, Result, SessionSummary};

pub fn parse(_summary: &SessionSummary) -> Result<ParsedSession> {
    Err(AppError::Unsupported(
        "DeepSeek Harness persistence root and format require local verification before parsing"
            .into(),
    ))
}
