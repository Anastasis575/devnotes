use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Config {
    edit_app: String,
    default_name: String,
    include_time: bool,
    group_by_date: bool,
    no_empty_adds_or_updates: bool,
}

impl Config {
    pub fn edit_app(&self) -> &String {
        &self.edit_app
    }
    pub fn default_name(&self) -> &String {
        &self.default_name
    }
    pub fn include_time(&self) -> bool {
        self.include_time
    }
    pub fn group_by_date(&self) -> bool {
        self.group_by_date
    }
    pub fn no_empty_adds_or_updates(&self) -> bool {
        self.no_empty_adds_or_updates
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

/// Return Option which is Some if it is not empty and None if it is
pub(crate) fn string_optional(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
