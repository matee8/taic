use std::{env, fs::File, io, path::PathBuf};

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("Failed to create history file.")]
    Create(#[from] io::Error),
    #[error("Failed to find cache directory for history.")]
    NoCacheDir,
}

#[inline]
pub fn locate_file() -> Result<PathBuf, HistoryError> {
    if let Ok(path) = env::var("LLMCLI_HISTORY_FILE") {
        Ok(PathBuf::from(path))
    } else if let Some(mut path) = dirs::cache_dir() {
        path.push("llmcli_history.txt");
        if !path.exists() {
            File::create(&path)?;
        }
        Ok(path)
    } else {
        Err(HistoryError::NoCacheDir)
    }
}
