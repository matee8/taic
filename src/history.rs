use alloc::borrow::Cow;
use std::{fs::File, io, path::PathBuf};

use thiserror::Error;

use crate::config::Config;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("Failed to create history file.")]
    Create(#[from] io::Error),
    #[error("Failed to find cache directory for history.")]
    NoCacheDir,
}

#[inline]
pub fn locate_file(config: &Config) -> Result<Cow<'_, PathBuf>, HistoryError> {
    if let Some(ref path) = config.history_path {
        return Ok(Cow::Borrowed(path));
    }

    if let Some(mut path) = dirs::cache_dir() {
        path.push("llmcli_history.txt");
        if !path.exists() {
            File::create(&path)?;
        }
        Ok(Cow::Owned(path))
    } else {
        Err(HistoryError::NoCacheDir)
    }
}
