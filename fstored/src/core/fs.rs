mod file_type;
mod hash;
mod lock;

use file_type::{mime_type, MimeType};
use lock::FileLock;

use anyhow::{bail, Result};
use log::{debug, error};
use std::{
    fs, io,
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};
use uuid::Uuid;

const OBJECTS_DIR: &str = "objects";
const PARTS_DIR: &str = "parts";

const OBJECT_PERMISSIONS: u32 = 0o640;

fn path_for_id(parent: &Path, id: &Uuid) -> PathBuf {
    const ID_SLICE_SIZE: usize = 2;
    const ID_SLICES: usize = 2;
    const CAPACITY: usize = ID_SLICE_SIZE * ID_SLICES + // Space for ID slices
        1 + ID_SLICES + // Space for separators
        36; // Space for ID

    let id = id.to_string();

    let mut result = parent.to_path_buf();
    result.reserve(CAPACITY);

    for i in 0..ID_SLICES {
        let start = i * ID_SLICE_SIZE;
        let end = start + ID_SLICE_SIZE;
        result.push(&id[start..end]);
    }

    result
}

pub fn remove(path: PathBuf) {
    match fs::remove_file(&path) {
        Ok(()) => debug!("Removed file '{}'", path.display()),
        Err(err) => {
            error!("Failed to remove file '{}': {}", path.display(), err)
        }
    };
}

pub struct Object {
    pub id: Uuid,
    pub hash: String,
    pub size: u64,
    pub r#type: String,
    pub subtype: String,
}

pub struct Part {
    id: Uuid,
    path: PathBuf,
    file: tokio::fs::File,
    lock: FileLock,
}

impl Part {
    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        let mut written = 0;

        while written < data.len() {
            written += match self.file.write(&data[written..]).await {
                Ok(bytes) => bytes,
                Err(err) => bail!(
                    "Failed to write data to part file '{}': {}",
                    self.id,
                    err
                ),
            }
        }

        Ok(())
    }
}

pub struct Filesystem {
    objects: PathBuf,
    parts: PathBuf,
}

impl Filesystem {
    pub fn new(home: &Path) -> Filesystem {
        let objects = home.join(OBJECTS_DIR);
        let parts = home.join(PARTS_DIR);

        Filesystem { objects, parts }
    }

    pub async fn commit(&self, part_id: Uuid) -> Result<Object> {
        let part = self.part_path(&part_id);
        let object = self.object_path(&part_id);

        let parent = object
            .parent()
            .expect("Object files should have a parent directory");

        fs::create_dir_all(parent)?;
        fs::rename(&part, &object)?;
        drop(part);

        let metadata = object.metadata()?;
        metadata.permissions().set_mode(OBJECT_PERMISSIONS);

        let MimeType { r#type, subtype } = mime_type(&object)?;

        Ok(Object {
            id: part_id,
            hash: hash::sha256sum(&object).await?,
            size: metadata.len(),
            r#type,
            subtype,
        })
    }

    fn object_path(&self, id: &Uuid) -> PathBuf {
        path_for_id(&self.objects, id)
    }

    pub async fn part(&self, id: Uuid) -> io::Result<Part> {
        let path = self.part_path(&id);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        let lock = lock::exclusive(file.as_raw_fd())?;

        Ok(Part {
            id,
            file,
            path,
            lock,
        })
    }

    fn part_path(&self, id: &Uuid) -> PathBuf {
        path_for_id(&self.parts, id)
    }
}
