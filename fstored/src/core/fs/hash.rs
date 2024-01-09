use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::{fs::File, io, path::Path};
use tokio::task;

pub async fn sha256sum(path: &Path) -> Result<String> {
    let buf = path.to_path_buf();
    task::spawn_blocking(move || sha256sum_sync(&buf)).await?
}

fn sha256sum_sync(path: &Path) -> Result<String> {
    let mut file = File::open(&path).with_context(|| {
        format!("Failed to open file '{}' for hashing", path.display())
    })?;

    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)
        .with_context(|| format!("Failed to hash file '{}'", path.display()))?;
    let hash = hasher.finalize();

    let mut buffer = [0u8; 64];
    let hex =
        base16ct::lower::encode_str(&hash, &mut buffer).map_err(|err| {
            anyhow!(
                "Failed to encode hash of '{}' into string: {}",
                path.display(),
                err
            )
        })?;

    Ok(String::from(hex))
}
