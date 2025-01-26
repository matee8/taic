use std::{fs, io, path::PathBuf};

use config::{Config, ConfigError, File};
use serde::Deserialize;
use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ConfigManagerError {
    #[error("Failed to locate project directories.")]
    ProjectDirs,
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

#[non_exhaustive]
#[derive(Deserialize)]
struct AppConfig {
    default_provider: String,
    default_model: String,
}

#[non_exhaustive]
pub struct ConfigManager {
    pub config_path: PathBuf,
}

impl ConfigManager {
    #[inline]
    pub fn new() -> Result<Self, ConfigManagerError> {
        let config_dir = dirs::config_dir()
            .ok_or(ConfigManagerError::ProjectDirs)?
            .join("llmcli");

        fs::create_dir_all(&config_dir)?;

        Ok(Self {
            config_path: config_dir.join("config.toml"),
        })
    }

    #[inline]
    pub fn init_default_config(&self) -> Result<(), ConfigManagerError> {
        if !self.config_path.exists() {
            const DEFAULT_CONFIG: &str = r#"
default_provider = "gemini"
default_model = "gemini-1.5-pro"
            "#;

            fs::write(&self.config_path, DEFAULT_CONFIG.trim())?;
        }
        Ok(())
    }

    fn load(self) -> Result<AppConfig, ConfigError> {
        Config::builder()
            .add_source(File::from(self.config_path))
            .build()?
            .try_deserialize()
    }
}
