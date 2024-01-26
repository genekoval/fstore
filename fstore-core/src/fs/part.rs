mod lock;

use lock::FileLock;

use super::create_directories;

use crate::error::{Error, Result};

use bytes::Bytes;
use futures::{pin_mut, Stream, TryStreamExt};
use log::debug;
use std::{
    collections::HashSet,
    os::fd::AsRawFd,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::fs::File;
use tokio_util::io::StreamReader;
use uuid::Uuid;

pub struct PartLock {
    id: Uuid,
    storage: Arc<Mutex<HashSet<Uuid>>>,
}

impl Drop for PartLock {
    fn drop(&mut self) {
        self.storage.lock().unwrap().remove(&self.id);
    }
}

#[derive(Debug, Default)]
pub struct PartLockSet {
    storage: Arc<Mutex<HashSet<Uuid>>>,
}

impl PartLockSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn lock(&self, id: &Uuid) -> Result<PartLock> {
        if self.storage.lock().unwrap().insert(*id) {
            Ok(PartLock {
                id: *id,
                storage: self.storage.clone(),
            })
        } else {
            Err(Error::WriteLock)
        }
    }
}

pub struct Part {
    internal_lock: PartLock,
    _external_lock: FileLock,
    path: PathBuf,
    file: tokio::fs::File,
}

impl Part {
    pub async fn open(
        id: &Uuid,
        path: PathBuf,
        locks: &PartLockSet,
    ) -> Result<Part> {
        let internal_lock = locks.lock(id)?;
        create_directories(&path)?;

        let file = File::options()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|err| {
                Error::Internal(format!(
                    "Failed to open part file '{}': {err}",
                    path.display()
                ))
            })?;

        let external_lock = lock::exclusive(file.as_raw_fd())?;

        Ok(Part {
            internal_lock,
            _external_lock: external_lock,
            path,
            file,
        })
    }

    pub fn id(&self) -> &Uuid {
        &self.internal_lock.id
    }

    pub async fn stream_to_file<S, E>(&mut self, stream: S) -> Result<u64>
    where
        S: Stream<Item = std::result::Result<Bytes, E>>,
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let stream = stream.map_err(std::io::Error::other);
        let reader = StreamReader::new(stream);
        pin_mut!(reader);

        let bytes = tokio::io::copy(&mut reader, &mut self.file)
            .await
            .map_err(|err| {
                Error::Internal(format!(
                    "Failed to copy stream data to part file '{}': {err}",
                    self.path.display()
                ))
            })?;

        debug!(
            "Wrote {bytes} byte{} to part file '{}'",
            match bytes {
                1 => "",
                _ => "s",
            },
            self.path.display()
        );

        Ok(bytes)
    }
}
