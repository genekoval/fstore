use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
pub struct Server {
    pub url: Url,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub servers: HashMap<String, Server>,
}

impl Config {
    pub fn read(path: Option<PathBuf>) -> Result<Config, String> {
        let Some(path) = path.or_else(find_config) else {
            return Ok(Default::default());
        };

        let data = fs::read_to_string(&path).map_err(|err| {
            format!("failed to read config file '{}': {err}", path.display())
        })?;

        toml::from_str(&data).map_err(|err| {
            format!(
                "failed to deserialize TOML config file '{}': {err}",
                path.display()
            )
        })
    }
}

fn find_config() -> Option<PathBuf> {
    search_xdg_config_home().or_else(search_home)
}

fn search_config_dir(dir: &Path) -> Option<PathBuf> {
    let path = dir.join("fstore/fstore.toml");
    if path.is_file() {
        return Some(path);
    }

    let path = dir.join("fstore.toml");
    if path.is_file() {
        return Some(path);
    }

    None
}

fn search_home() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    let home = Path::new(&home);

    let config = home.join(".config");
    if let Some(path) = search_config_dir(&config) {
        return Some(path);
    }

    let path = home.join(".fstore.toml");
    if path.is_file() {
        return Some(path);
    }

    None
}

fn search_xdg_config_home() -> Option<PathBuf> {
    let config = env::var_os("XDG_CONFIG_HOME")?;
    let path = Path::new(&config);

    search_config_dir(path)
}
