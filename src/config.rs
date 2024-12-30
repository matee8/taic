use std::{env, fs, fs::File, path::PathBuf};

use futures::io;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::{de, ser};

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    De(#[from] de::Error),
    #[error("{0}")]
    Ser(#[from] ser::Error),
    #[error("Config directory not found.")]
    NotFound,
}

#[non_exhaustive]
#[derive(Deserialize, Serialize)]
pub struct ApiKeys {
    pub gemini: Option<String>,
}

#[non_exhaustive]
#[derive(Deserialize, Serialize)]
pub struct DefaultModels {
    pub gemini: Option<String>,
}

#[non_exhaustive]
#[derive(Deserialize, Serialize, Default)]
pub struct Config {
    pub default_chatbot: Option<String>,
    pub default_models: Option<DefaultModels>,
    pub api_keys: Option<ApiKeys>,
    pub session_path: Option<PathBuf>,
    pub history_path: Option<PathBuf>,
}

impl Config {
    #[inline]
    pub fn load(cli_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let config_path = match Self::get_file_path(cli_path) {
            Ok(path) => path,
            Err(ConfigError::NotFound) => {
                return Ok(Self::default());
            }
            Err(err) => {
                return Err(err);
            }
        };

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(config_path)?;

        if config_str.trim().is_empty() {
            return Ok(Self::default());
        }

        Ok(toml::from_str(&config_str)?)
    }

    #[inline]
    pub fn save(&self, cli_path: Option<PathBuf>) -> Result<(), ConfigError> {
        let config_path = Self::get_file_path(cli_path)?;
        let config_str = toml::to_string(self)?;
        fs::write(config_path, config_str)?;
        Ok(())
    }

    fn get_file_path(
        cli_path: Option<PathBuf>,
    ) -> Result<PathBuf, ConfigError> {
        if let Some(path) = cli_path {
            return Ok(path);
        }

        if let Ok(env_path) = env::var("LLMCLI_CONFIG_PATH") {
            return Ok(PathBuf::from(env_path));
        }

        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("llmcli/config.toml");
            if let Some(parent) = config_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            if !config_path.exists() {
                File::create(&config_path)?;
            }
            return Ok(config_path);
        }

        Err(ConfigError::NotFound)
    }
}
