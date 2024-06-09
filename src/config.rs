use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Config {
    edit_app: Option<String>,
    default_name: Option<String>,
    include_time: Option<bool>,
}

impl Config {
    pub fn edit_app(&self) -> &Option<String> {
        &self.edit_app
    }
    pub fn default_name(&self) -> &Option<String> {
        &self.default_name
    }
    pub fn include_time(&self) -> Option<bool> {
        self.include_time
    }
}

/// Get  the configuration
pub(crate) fn get_or_create_config<K: AsRef<OsStr> + ?Sized>(exe_dir: &K) -> Result<Config> {
    let buf = PathBuf::from(exe_dir).join("config.toml");
    let config_path = buf.to_str().unwrap();
    if !Path::new(&config_path).exists() {
        fs::write(&config_path, toml::to_string(&Config::default())?)?;
        return Err(anyhow!("Config doesn't exist... default generated"));
    }
    let toml_str = fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(toml_str.as_str())?;
    Ok(config)
}