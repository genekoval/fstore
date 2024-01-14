use serde::{Deserialize, Serialize};
use serde_yaml as yaml;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    pub connection: HashMap<String, String>,
    pub max_connections: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Http {
    pub listen: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub http: Http,
    pub home: PathBuf,
    pub database: Database,
}

pub fn read(path: &Path) -> Result<Config, String> {
    let data = fs::read_to_string(path).map_err(|err| {
        format!("Failed to read config file '{}': {err}", path.display())
    })?;

    yaml::from_str(&data).map_err(|err| {
        format!(
            "Failed to deserialize YAML config file '{}': {err}",
            path.canonicalize()
                .ok()
                .as_deref()
                .unwrap_or(path)
                .display()
        )
    })
}
