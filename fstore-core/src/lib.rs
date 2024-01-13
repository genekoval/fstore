mod db;
mod error;
mod fs;

pub use db::Database;
pub use fs::{Filesystem, Part};

use crate::error::Result;

use fstore::{Bucket, Object, ObjectError, RemoveResult, StoreTotals};
use log::info;
use uuid::Uuid;

pub struct ObjectStore {
    database: Database,
    filesystem: Filesystem,
}

impl ObjectStore {
    pub fn new(database: Database, filesystem: Filesystem) -> ObjectStore {
        ObjectStore {
            database,
            filesystem,
        }
    }

    pub async fn add_bucket(&self, name: &str) -> Result<Bucket> {
        Ok(self.database.create_bucket(name).await?.into())
    }

    pub async fn commit_part(
        &self,
        bucket_id: &Uuid,
        part_id: &Uuid,
    ) -> Result<Object> {
        let metadata = self.filesystem.commit(part_id).await?;

        Ok(self
            .database
            .add_object(
                bucket_id,
                &metadata.id,
                metadata.hash.as_str(),
                metadata.size.try_into().unwrap(),
                metadata.r#type.as_str(),
                metadata.subtype.as_str(),
            )
            .await?
            .into())
    }

    pub async fn get_bucket(&self, name: &str) -> Result<Bucket> {
        Ok(self.database.fetch_bucket(name).await?.into())
    }

    pub async fn get_buckets(&self) -> Result<Vec<Bucket>> {
        Ok(self
            .database
            .fetch_buckets()
            .await?
            .into_iter()
            .map(|bucket| bucket.into())
            .collect())
    }

    pub async fn get_errors(&self) -> Result<Vec<ObjectError>> {
        Ok(self
            .database
            .get_errors()
            .await?
            .into_iter()
            .map(|errors| errors.into())
            .collect())
    }

    pub async fn get_object_metadata(
        &self,
        bucket_id: &Uuid,
        object_id: &Uuid,
    ) -> Result<Option<Object>> {
        Ok(self
            .database
            .get_object(bucket_id, object_id)
            .await?
            .map(|object| object.into()))
    }

    pub async fn get_part(&self, part_id: Option<&Uuid>) -> Result<Part> {
        let generated;
        let id = match part_id {
            Some(id) => id,
            None => {
                generated = Uuid::new_v4();
                &generated
            }
        };

        Ok(self.filesystem.part(id).await?)
    }

    pub async fn get_totals(&self) -> Result<StoreTotals> {
        Ok(self.database.fetch_store_totals().await?.into())
    }

    pub async fn prune(&self) -> Result<Vec<Object>> {
        let mut tx = self.database.begin().await?;
        let objects = tx.remove_orphan_objects().await?;

        for object in &objects {
            self.filesystem.remove_object(&object.object_id);
        }

        tx.commit().await?;

        info!("Pruned {} objects", objects.len());

        Ok(objects.into_iter().map(|object| object.into()).collect())
    }

    pub async fn remove_bucket(&self, bucket_id: &Uuid) -> Result<()> {
        Ok(self.database.remove_bucket(bucket_id).await?)
    }

    pub async fn remove_object(
        &self,
        bucket_id: &Uuid,
        object_id: &Uuid,
    ) -> Result<Option<Object>> {
        Ok(self
            .database
            .remove_object(bucket_id, object_id)
            .await?
            .map(|object| object.into()))
    }

    pub async fn remove_objects(
        &self,
        bucket_id: &Uuid,
        objects: &[Uuid],
    ) -> Result<RemoveResult> {
        Ok(self
            .database
            .remove_objects(bucket_id, objects)
            .await?
            .into())
    }

    pub async fn rename_bucket(
        &self,
        bucket_id: &Uuid,
        new_name: &str,
    ) -> Result<()> {
        Ok(self.database.rename_bucket(bucket_id, new_name).await?)
    }

    pub async fn shutdown(self) {
        self.database.close().await
    }
}
