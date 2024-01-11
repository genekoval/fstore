mod lock;

use lock::FileLock;

use crate::error::{internal, Error, Result};

use std::{
    collections::HashSet,
    os::fd::AsRawFd,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::io::AsyncWriteExt;
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

pub struct PartLockSet {
    storage: Arc<Mutex<HashSet<Uuid>>>,
}

impl PartLockSet {
    pub fn new() -> PartLockSet {
        PartLockSet {
            storage: Arc::new(Mutex::new(HashSet::new())),
        }
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
    id: Uuid,
    path: PathBuf,
    file: tokio::fs::File,
    internal_lock: PartLock,
    external_lock: FileLock,
}

impl Part {
    pub fn new(
        id: &Uuid,
        path: PathBuf,
        file: tokio::fs::File,
        lock: PartLock,
    ) -> Result<Part> {
        let external_lock = lock::exclusive(file.as_raw_fd())?;

        Ok(Part {
            id: *id,
            path,
            file,
            internal_lock: lock,
            external_lock,
        })
    }

    async fn write(&mut self, data: &[u8]) -> Result<()> {
        let mut written = 0;

        while written < data.len() {
            written += match self.file.write(&data[written..]).await {
                Ok(bytes) => bytes,
                Err(err) => internal!(
                    "Failed to write data to part file '{}': {}",
                    self.id,
                    err
                ),
            }
        }

        Ok(())
    }
}
