use std::{env, ffi::OsStr, fs, path::PathBuf};

use futures::io;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Message, Role};

#[non_exhaustive]
#[derive(Serialize, Deserialize, Default)]
pub struct Session {
    pub messages: Vec<Message>,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to create directory: {0}.")]
    CreateDir(io::Error),
    #[error("Failed to get data directory.")]
    DataDir,
    #[error("Failed to serialize or deserialize session: {0}.")]
    Serialize(#[from] serde_json::Error),
    #[error("Failed to write file: {0}.")]
    WriteFile(io::Error),
    #[error("Failed to read file: {0}.")]
    ReadFile(io::Error),
    #[error("Failed to read directory: {0}.")]
    ReadDir(io::Error),
    #[error("Session not found.")]
    NotFound,
    #[error("Failed to delete file: {0}.")]
    DeleteFile(io::Error),
}

impl Session {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    #[inline]
    pub fn save(&self, filename: &str) -> Result<(), SessionError> {
        let session_dir = Self::get_dir_path()?;
        let file_path = session_dir.join(filename).with_extension("json");
        let serialized = serde_json::to_string(self)?;

        fs::write(&file_path, serialized).map_err(SessionError::WriteFile)?;

        Ok(())
    }

    #[inline]
    pub fn load(filename: &str) -> Result<Self, SessionError> {
        let session_dir = Self::get_dir_path()?;
        let file_path = session_dir.join(filename).with_extension("json");
        let file_content =
            fs::read_to_string(file_path).map_err(SessionError::ReadFile)?;
        let session: Self = serde_json::from_str(&file_content)?;

        Ok(session)
    }

    #[inline]
    pub fn list_all() -> Result<Vec<String>, SessionError> {
        let session_dir = Self::get_dir_path()?;
        let entries =
            fs::read_dir(session_dir).map_err(SessionError::ReadDir)?;
        let session_files: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|file| file.path().extension() == Some(OsStr::new("json")))
            .map(|file| {
                file.file_name()
                    .to_string_lossy()
                    .trim_end_matches(".json")
                    .to_owned()
            })
            .collect();

        Ok(session_files)
    }

    #[inline]
    pub fn delete(filename: &str) -> Result<(), SessionError> {
        let session_dir = Self::get_dir_path()?;
        let file_path = session_dir.join(filename).with_extension("json");

        if file_path.exists() {
            fs::remove_file(file_path).map_err(SessionError::DeleteFile)?;

            Ok(())
        } else {
            Err(SessionError::NotFound)
        }
    }

    #[inline]
    pub fn add_message(&mut self, role: Role, content: String) {
        self.messages.push(Message::new(role, content));
    }

    fn get_dir_path() -> Result<PathBuf, SessionError> {
        let session_dir = if let Ok(env_dir) = env::var("LLMCLI_SESSION_DIR") {
            PathBuf::from(env_dir)
        } else {
            let data_dir = dirs::data_dir().ok_or(SessionError::DataDir)?;
            data_dir.join("llmcli_sessions")
        };

        if !session_dir.exists() {
            fs::create_dir_all(&session_dir)
                .map_err(SessionError::CreateDir)?;
        }

        Ok(session_dir)
    }
}
