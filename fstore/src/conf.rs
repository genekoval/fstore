use serde::{Deserialize, Serialize};
use serde_yaml as yaml;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::ErrorKind,
    path::{Path, PathBuf},
};
use url::Url;

fn find_cache_dir() -> Option<PathBuf> {
    if let Some(cache) = env::var_os("XDG_CACHE_HOME") {
        return Some(Path::new(&cache).join("fstore"));
    }

    if let Some(home) = env::var_os("HOME") {
        let mut cache = Path::new(&home).join(".cache");
        cache.push("fstore");
        return Some(cache);
    }

    None
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Global {
    pub server: Option<String>,
}

impl Global {
    fn merge(&mut self, other: Global) {
        if self.server.is_none() {
            self.server = other.server;
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Server {
    pub url: Url,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub cache: Option<PathBuf>,
    pub global: Option<Global>,
    pub servers: HashMap<String, Server>,
}

impl Config {
    fn cache_dir(&self) -> Result<PathBuf, String> {
        match self.cache.clone().or_else(find_cache_dir) {
            Some(cache) => Ok(cache),
            None => {
                Err("Could not determine a cache directory location".into())
            }
        }
    }

    pub fn server(
        &self,
        name: Option<&str>,
    ) -> Result<Option<&Server>, String> {
        let server = match name {
            Some(name) => name,
            None => {
                match self.global.as_ref().and_then(|g| g.server.as_deref()) {
                    Some(server) => server,
                    None => return Ok(None),
                }
            }
        };

        match self.servers.get(server.trim()) {
            Some(server) => Ok(Some(server)),
            None => {
                Err(format!("Server '{server}' not defined in config file"))
            }
        }
    }

    fn global_file(&self) -> Result<PathBuf, String> {
        Ok(self.cache_dir()?.join("global.yml"))
    }

    fn merge_cache(&mut self) -> Result<(), String> {
        if let Some(cached) = self.read_cache()? {
            match &mut self.global {
                Some(global) => global.merge(cached),
                None => self.global = Some(cached),
            }
        }

        Ok(())
    }

    fn read_cache(&self) -> Result<Option<Global>, String> {
        let path = self.global_file()?;

        let data = match fs::read_to_string(&path) {
            Ok(data) => data,
            Err(err) => match err.kind() {
                ErrorKind::NotFound => return Ok(None),
                _ => {
                    return Err(format!(
                        "Failed to read cached settings at '{}': {err}",
                        path.display()
                    ))
                }
            },
        };

        Ok(Some(yaml::from_str(&data).map_err(|err| {
            format!(
                "Failed to deserialize cached config file '{}': {err}",
                path.display()
            )
        })?))
    }

    pub fn set_server(&self, server: &str) -> Result<(), String> {
        let mut global = self.read_cache()?.unwrap_or_default();
        global.server = Some(server.into());

        self.write_cache(&global)
    }

    fn write_cache(&self, global: &Global) -> Result<(), String> {
        let path = self.global_file()?;

        let parent = path
            .parent()
            .expect("Cached file should have a parent directory");

        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create cache directory '{}': {err}",
                parent.display()
            )
        })?;

        let file = File::create(&path).map_err(|err| {
            format!(
                "Failed to create global server file '{}': {err}",
                path.display()
            )
        })?;

        yaml::to_writer(file, global).map_err(|err| {
            format!(
                "Failed to write cache data to file '{}': {err}",
                path.display()
            )
        })
    }
}

pub fn read(path: &Path) -> Result<Config, String> {
    let data = fs::read_to_string(path).map_err(|err| {
        format!("Failed to read config file '{}': {err}", path.display())
    })?;

    let mut config: Config = yaml::from_str(&data).map_err(|err| {
        format!(
            "Failed to deserialize YAML config file '{}': {err}",
            path.canonicalize()
                .ok()
                .as_deref()
                .unwrap_or(path)
                .display()
        )
    })?;

    config.merge_cache()?;

    Ok(config)
}
