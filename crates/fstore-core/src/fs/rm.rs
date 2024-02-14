use super::{path_for_id, ID_SLICES};

use crate::error::{internal, Error, Result};

use log::{debug, error, trace};
use std::{fs, io::ErrorKind, path::Path};
use uuid::Uuid;
use walkdir::WalkDir;

use std::path::PathBuf;
use tokio::task;

pub async fn remove_extraneous(src: &Path, dest: &Path) -> Result<()> {
    let source = src.to_owned();
    let destination = dest.to_owned();

    let result = task::spawn_blocking(move || {
        blocking::remove_extraneous(&source, &destination)
    })
    .await;

    let result = match result {
        Ok(result) => result,
        Err(_) => internal!(
            "failed to remove extraneous object files from '{}': \
            background task failed",
            dest.display()
        ),
    };

    result.map_err(|err| {
        Error::Internal(format!(
            "failed to remove extraneous object files from '{}': {err}",
            dest.display()
        ))
    })
}

pub async fn remove_files(paths: Vec<PathBuf>) -> Result<()> {
    let len = paths.len();

    let result = task::spawn_blocking(move || -> Result<()> {
        for path in paths {
            blocking::remove(&path)?;
        }

        Ok(())
    })
    .await;

    match result {
        Ok(result) => result,
        Err(_) => {
            internal!("failed to remove {} files: background task failed", len)
        }
    }
}

mod blocking {
    use super::*;

    pub fn remove(path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => debug!("Removed file '{}'", path.display()),
            Err(err) => match err.kind() {
                ErrorKind::NotFound => (),
                _ => internal!(
                    "failed to remove file '{}': {}",
                    path.display(),
                    err
                ),
            },
        }

        let mut dir = path;

        for _ in 0..ID_SLICES {
            dir = dir.parent().unwrap();

            if dir.read_dir().unwrap().next().is_some() {
                break;
            }

            match fs::remove_dir(dir) {
                Ok(()) => trace!("Removed empty directory '{}'", dir.display()),
                Err(err) => error!(
                    "Failed to remove empty directory '{}': {err}",
                    dir.display()
                ),
            }
        }

        Ok(())
    }

    pub fn remove_extraneous(src: &Path, dest: &Path) -> Result<()> {
        if !dest.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(dest).into_iter() {
            let entry =
                entry.map_err(|err| Error::Internal(format!("{err}")))?;

            if entry.file_type().is_dir() {
                continue;
            }

            if !entry.file_type().is_file() {
                debug!("Removing '{}': not a file", entry.path().display());
                remove(entry.path())?;
                continue;
            }

            let Some(name) = entry.file_name().to_str() else {
                debug!(
                    "Removing '{}': name is not valid UTF-8",
                    entry.path().display()
                );
                remove(entry.path())?;
                continue;
            };

            let Some(id) = Uuid::try_parse(name).ok() else {
                debug!(
                    "Removing '{}': name is not valid UUID",
                    entry.path().display()
                );
                remove(entry.path())?;
                continue;
            };

            if !path_for_id(src, &id).exists() {
                debug!(
                    "Removing '{}': not present in source directory",
                    entry.path().display()
                );
                remove(entry.path())?;
                continue;
            }

            trace!("Keeping file '{}'", entry.path().display());
        }

        Ok(())
    }
}
