use std::{env, fs, path::PathBuf};

use futures::io;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::{de, ser};

#[non_exhaustive]
#[derive(Deserialize, Serialize)]
pub struct ApiKeys {
    pub gemini: Option<String>,
}

#[non_exhaustive]
#[derive(Deserialize, Serialize)]
pub struct Config {
    pub default_chatbot: String,
    pub default_model: String,
    pub api_keys: ApiKeys,
}

impl Default for Config {
    #[inline]
    fn default() -> Self {
        Self {
            default_chatbot: "gemini".to_owned(),
            default_model: "gemini-1.5-flash".to_owned(),
            api_keys: ApiKeys { gemini: None },
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Toml(#[from] de::Error),
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ConfigSaveError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Toml(#[from] ser::Error),
}

impl Config {
    fn get_file_path() -> io::Result<PathBuf> {
        if let Ok(config_path) = env::var("LLMCLI_CONFIG_PATH") {
            return Ok(PathBuf::from(config_path));
        }

        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("llmcli/config.toml");
            if let Some(parent) = config_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            return Ok(config_path);
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Config directory not found",
        ))
    }

    #[inline]
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, ConfigLoadError> {
        let config_path = if let Some(config_path) = config_path {
            config_path
        } else {
            Self::get_file_path()?
        };
        let config_str = fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_str)?)
    }

    #[inline]
    pub fn save(
        &self,
        config_path: Option<PathBuf>,
    ) -> Result<(), ConfigSaveError> {
        let config_path = if let Some(config_path) = config_path {
            config_path
        } else {
            Self::get_file_path()?
        };
        let config_str = toml::to_string(self)?;
        fs::write(config_path, config_str)?;
        Ok(())
    }
}
