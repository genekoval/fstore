use crate::error::{internal, Error, Result};

use sha2::{Digest, Sha256};
use std::{fs::File, io, path::Path};
use tokio::task;

pub async fn sha256sum(path: &Path) -> Result<String> {
    let buf = path.to_path_buf();
    task::spawn_blocking(move || sha256sum_sync(&buf))
        .await
        .map_err(|err| {
            Error::Internal(format!(
                "Task failed while hashing file '{}': {err}",
                path.display()
            ))
        })?
}

fn sha256sum_sync(path: &Path) -> Result<String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) => internal!(
            "Failed to open file '{}' for hashing: {err}",
            path.display()
        ),
    };

    let mut hasher = Sha256::new();
    if let Err(err) = io::copy(&mut file, &mut hasher) {
        internal!("Failed to hash file '{}': {err}", path.display());
    }
    let hash = hasher.finalize();

    let mut buffer = [0u8; 64];
    let hex = match base16ct::lower::encode_str(&hash, &mut buffer) {
        Ok(hex) => hex,
        Err(err) => internal!(
            "Failed to encode hash of '{}' into string: {err}",
            path.display()
        ),
    };

    Ok(String::from(hex))
}
