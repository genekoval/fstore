use axum_unix::Endpoint;
use fstore_core::DatabaseConfig;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use timber::Sink;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub archive: Option<PathBuf>,

    pub database: DatabaseConfig,

    pub home: PathBuf,

    pub http: Http,

    #[serde(default)]
    pub log: Log,

    pub user: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Http {
    pub listen: Vec<Endpoint>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Log {
    #[serde(default = "Log::default_level")]
    pub level: LevelFilter,

    #[serde(default)]
    pub sink: Sink,
}

impl Log {
    fn default_level() -> LevelFilter {
        LevelFilter::Info
    }
}

impl Default for Log {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            sink: Default::default(),
        }
    }
}

pub fn read(path: &Path) -> Result<Config, String> {
    let data = fs::read_to_string(path).map_err(|err| {
        format!("Failed to read config file '{}': {err}", path.display())
    })?;

    toml::from_str(&data).map_err(|err| {
        format!(
            "Failed to deserialize TOML config file '{}': {err}",
            path.canonicalize()
                .ok()
                .as_deref()
                .unwrap_or(path)
                .display()
        )
    })
}
