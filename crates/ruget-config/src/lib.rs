use std::path::PathBuf;

pub use clap::ArgMatches;
pub use config::Config as RuGetConfig;
use config::{ConfigError, Environment, File};
use ruget_common::miette::{self, Diagnostic, Result};
use ruget_common::thiserror::{self, Error};

pub use ruget_config_derive::*;

pub trait RuGetConfigLayer {
    fn layer_config(
        &mut self,
        _matches: &ArgMatches,
        _config: &RuGetConfig,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Diagnostic, Error)]
pub enum RuGetConfigError {
    #[error(transparent)]
    #[diagnostic(code(config::error))]
    ConfigError(#[from] ConfigError),

    #[error(transparent)]
    #[diagnostic(code(config::parse_error))]
    ConfigParseError(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub struct RuGetConfigOptions {
    global: bool,
    env: bool,
    pkg_root: Option<PathBuf>,
    global_config_file: Option<PathBuf>,
}

impl Default for RuGetConfigOptions {
    fn default() -> Self {
        RuGetConfigOptions {
            global: true,
            env: true,
            pkg_root: None,
            global_config_file: None,
        }
    }
}

impl RuGetConfigOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global(mut self, global: bool) -> Self {
        self.global = global;
        self
    }

    pub fn env(mut self, env: bool) -> Self {
        self.env = env;
        self
    }

    pub fn pkg_root(mut self, root: Option<PathBuf>) -> Self {
        self.pkg_root = root;
        self
    }

    pub fn global_config_file(mut self, file: Option<PathBuf>) -> Self {
        self.global_config_file = file;
        self
    }

    pub fn load(self) -> Result<RuGetConfig, RuGetConfigError> {
        let mut c = RuGetConfig::new();
        if self.global {
            if let Some(config_file) = self.global_config_file {
                let path = config_file.display().to_string();
                c.merge(File::with_name(&path[..]).required(false))
                    .map_err(RuGetConfigError::ConfigError)?;
            }
        }
        if self.env {
            c.merge(Environment::with_prefix("ruget_config"))
                .map_err(RuGetConfigError::ConfigError)?;
        }
        if let Some(root) = self.pkg_root {
            c.merge(File::with_name(&root.join("rugetrc").display().to_string()).required(false))
                .map_err(RuGetConfigError::ConfigError)?;
            c.merge(File::with_name(&root.join(".rugetrc").display().to_string()).required(false))
                .map_err(RuGetConfigError::ConfigError)?;
            c.merge(
                File::with_name(&root.join("rugetrc.toml").display().to_string()).required(false),
            )
            .map_err(RuGetConfigError::ConfigError)?;
            c.merge(
                File::with_name(&root.join(".rugetrc.toml").display().to_string()).required(false),
            )
            .map_err(RuGetConfigError::ConfigError)?;
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use std::fs;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn env_configs() -> Result<()> {
        let dir = tempdir()?;
        env::set_var("RUGET_CONFIG_STORE", dir.path().display().to_string());
        let config = RuGetConfigOptions::new().global(false).load()?;
        env::remove_var("RUGET_CONFIG_STORE");
        assert_eq!(config.get_str("store")?, dir.path().display().to_string());
        Ok(())
    }

    #[test]
    fn global_config() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("rugetrc.toml");
        fs::write(&file, "store = \"hello world\"")?;
        let config = RuGetConfigOptions::new()
            .env(false)
            .global_config_file(Some(file))
            .load()?;
        assert_eq!(config.get_str("store")?, String::from("hello world"));
        Ok(())
    }

    #[test]
    fn missing_config() -> Result<()> {
        let config = RuGetConfigOptions::new().global(false).env(false).load()?;
        assert!(config.get_str("store").is_err());
        Ok(())
    }
}
