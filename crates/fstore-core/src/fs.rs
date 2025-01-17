mod file_type;
mod hash;
mod part;
mod rm;

pub use part::Part;
pub use tokio::fs::File;

use file_type::{mime_type, MimeType};
use part::PartLockSet;

use crate::error::{Error, Result};

use log::debug;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    result,
};
use uuid::Uuid;

const ID_SLICE_SIZE: usize = 2;
const ID_SLICES: usize = 2;

const OBJECTS_DIR: &str = "objects";
const PARTS_DIR: &str = "parts";

const OBJECT_PERMISSIONS: u32 = 0o640;

async fn check(path: &Path, hash: &str) -> result::Result<(), String> {
    if !path.exists() {
        return Err(format!("file '{}' does not exist", path.display()));
    }

    match hash::sha256sum(path).await {
        Ok(result) => {
            if result == hash {
                Ok(())
            } else {
                Err(format!("hash mismatch: {result}"))
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

fn create_directories(file: &Path) -> Result<()> {
    let parent = file.parent().ok_or_else(|| {
        Error::Internal(format!(
            "No parent directory for file '{}'",
            file.display()
        ))
    })?;

    fs::create_dir_all(parent).map_err(|err| {
        Error::Internal(format!(
            "Failed to create parent directories \
            for file '{}': {err}",
            parent.display()
        ))
    })?;

    Ok(())
}

fn path_for_id(parent: &Path, id: &Uuid) -> PathBuf {
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

    result.push(id);

    result
}

#[derive(Debug)]
pub struct Object {
    pub id: Uuid,
    pub hash: String,
    pub size: u64,
    pub r#type: String,
    pub subtype: String,
}

#[derive(Debug)]
pub struct Filesystem {
    objects: PathBuf,
    parts: PathBuf,
    locked_parts: PartLockSet,
}

impl Filesystem {
    pub fn new(home: &Path) -> Self {
        Self {
            objects: home.join(OBJECTS_DIR),
            parts: home.join(PARTS_DIR),
            locked_parts: PartLockSet::new(),
        }
    }

    pub async fn check(
        &self,
        object_id: &Uuid,
        hash: &str,
    ) -> result::Result<(), String> {
        let path = self.object_path(object_id);
        check(&path, hash).await
    }

    pub async fn commit(&self, part_id: &Uuid) -> Result<Object> {
        let _lock = self.locked_parts.lock(part_id);
        let object = self.move_part(part_id)?;

        let metadata = object.metadata().map_err(|err| {
            Error::Internal(format!(
                "Failed to fetch metadata for object file '{}': {err}",
                object.display()
            ))
        })?;
        metadata.permissions().set_mode(OBJECT_PERMISSIONS);

        let MimeType { r#type, subtype } = mime_type(&object)?;

        Ok(Object {
            id: *part_id,
            hash: hash::sha256sum(&object).await?,
            size: metadata.len(),
            r#type,
            subtype,
        })
    }

    pub async fn copy(
        &self,
        object_id: &Uuid,
        destination: &Path,
        hash: &str,
    ) -> result::Result<(), String> {
        let objects = destination.join(OBJECTS_DIR);
        let destination = path_for_id(&objects, object_id);

        match check(&destination, hash).await {
            Ok(()) => return Ok(()),
            Err(err) => debug!(
                "Copying object ({object_id}) to '{}': {err}",
                destination.display()
            ),
        }

        let source = self.object_path(object_id);

        create_directories(&destination)
            .map_err(|err| format!("failed to copy object file: {err}"))?;

        tokio::fs::copy(&source, &destination)
            .await
            .map_err(|err| {
                format!(
                    "failed to copy object file from '{}' to '{}': {err}",
                    source.display(),
                    destination.display()
                )
            })?;

        Ok(())
    }

    fn move_part(&self, part_id: &Uuid) -> Result<PathBuf> {
        let part = self.part_path(part_id);
        let object = self.object_path(part_id);

        create_directories(&object)?;
        fs::rename(&part, &object).map_err(|err| {
            Error::Internal(format!(
                "Failed to move part file to objects directory \
                ({} -> {}): {err}",
                &part.display(),
                &object.display()
            ))
        })?;

        Ok(object)
    }

    pub async fn object(&self, id: &Uuid) -> Result<File> {
        let path = self.object_path(id);
        let file = File::open(&path).await.map_err(|err| {
            Error::Internal(format!(
                "Failed to open object file '{}': {err}",
                path.display()
            ))
        })?;

        Ok(file)
    }

    fn object_path(&self, id: &Uuid) -> PathBuf {
        path_for_id(&self.objects, id)
    }

    pub async fn part(&self, id: &Uuid) -> Result<Part> {
        Part::open(id, self.part_path(id), &self.locked_parts).await
    }

    fn part_path(&self, id: &Uuid) -> PathBuf {
        path_for_id(&self.parts, id)
    }

    pub async fn remove_extraneous(&self, dest: &Path) -> Result<()> {
        let dest = dest.join(OBJECTS_DIR);
        rm::remove_extraneous(&self.objects, &dest).await
    }

    pub async fn remove_objects<'a, I>(&self, objects: I) -> Result<()>
    where
        I: Iterator<Item = &'a Uuid>,
    {
        let paths = objects.map(|id| self.object_path(id)).collect();
        rm::remove_files(paths).await
    }
}
