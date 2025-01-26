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
        let cfg_dir = dirs::config_dir()
            .ok_or(ConfigManagerError::ProjectDirs)?
            .join("llmcli");

        fs::create_dir_all(&cfg_dir)?;

        Ok(Self {
            config_path: cfg_dir.join("config.toml"),
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

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "We want panics on failure to fail the test cases."
)]
mod tests {
    use std::env;

    use assert_fs::TempDir;

    use super::ConfigManager;

    #[test]
    fn config_init_creates_config_file() {
        let tmp_dir = TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", tmp_dir.path());

        let cfg_mgr = ConfigManager::new().unwrap();
        cfg_mgr.init_default_config().unwrap();

        assert!(tmp_dir.join("llmcli").join("config.toml").exists());
    }

    #[test]
    fn config_load_loads_correct_values() {
        let tmp_dir = TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", tmp_dir.path());

        let cfg_mgr = ConfigManager::new().unwrap();
        std::fs::write(
            &cfg_mgr.config_path,
            r#"
default_provider = "test-provider"
default_model = "test-model"
            "#
            .trim(),
        )
        .unwrap();

        let cfg = cfg_mgr.load().unwrap();

        assert_eq!(cfg.default_provider, "test-provider");
        assert_eq!(cfg.default_model, "test-model");
    }

    #[test]
    fn config_manager_creates_missing_dir() {
        let tmp_dir = TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", tmp_dir.path());

        let cfg_mgr = ConfigManager::new().unwrap();
        println!(
            "{:?}, {:?}",
            env::var("XDG_CONFIG_HOME"),
            cfg_mgr.config_path
        );
        assert!(cfg_mgr.config_path.parent().unwrap().exists());
    }

    #[test]
    fn config_has_correct_default_values() {
        let tmp_dir = TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", tmp_dir.path());

        let cfg_mgr = ConfigManager::new().unwrap();
        cfg_mgr.init_default_config().unwrap();

        let cfg = cfg_mgr.load().unwrap();

        assert_eq!(cfg.default_provider, "gemini");
        assert_eq!(cfg.default_model, "gemini-1.5-pro");
    }
}
