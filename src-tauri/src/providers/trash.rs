use crate::domain::{AppError, Result};
use std::path::Path;

pub fn move_to_trash(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(AppError::Invalid("待移入垃圾篓的容器已不存在".into()));
    }
    trash::delete(path).map_err(|error| AppError::Invalid(format!("macOS 垃圾篓操作失败：{error}")))
}
