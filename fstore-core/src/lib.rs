mod db;
mod error;
mod fs;
mod model;

pub use pgtools::{
    ConnectionParameters as DbConnection, Database as DbSupport,
};

pub use error::Error;
pub use fs::{File, Part};
pub use model::*;

pub use sqlx::postgres::PgPoolOptions as DbPoolOptions;
use sqlx::postgres::PgPoolOptions;

use crate::{
    db::Database,
    error::{OptionNotFound, Result},
    fs::Filesystem,
};

use fstore::{Bucket, Object, ObjectError, RemoveResult, StoreTotals};
use log::info;
use pgtools::{PgDump, PgRestore, Psql};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    result,
};
use uuid::Uuid;

const DATABASE_DUMP_FILENAME: &str = "fstore.dump";

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub connection: DbConnection,

    pub max_connections: Option<u32>,

    #[serde(default)]
    pub psql: Psql,

    #[serde(default)]
    pub pg_dump: PgDump,

    #[serde(default)]
    pub pg_restore: PgRestore,

    pub sql_directory: PathBuf,
}

#[derive(Clone, Copy, Debug)]
pub struct StoreOptions<'a> {
    pub version: Version,
    pub database: &'a DatabaseConfig,
    pub home: &'a Path,
    pub archive: &'a Option<PathBuf>,
}

pub struct ObjectStore {
    about: About,
    database: Database,
    db_support: DbSupport,
    filesystem: Filesystem,
    archive: Option<PathBuf>,
}

impl ObjectStore {
    pub async fn new(
        StoreOptions {
            version,
            database,
            home,
            archive,
        }: StoreOptions<'_>,
    ) -> result::Result<Self, String> {
        let mut pool = PgPoolOptions::new();

        if let Some(max_connections) = database.max_connections {
            pool = pool.max_connections(max_connections);
        }

        let pool = pool
            .connect(database.connection.as_url().as_str())
            .await
            .map_err(|err| {
                format!("failed to establish database connection: {err}")
            })?;

        let db_support = DbSupport::new(
            version.number,
            pgtools::Options {
                connection: &database.connection,
                psql: &database.psql,
                pg_dump: &database.pg_dump,
                pg_restore: &database.pg_restore,
                sql_directory: &database.sql_directory,
            },
        )?;

        Ok(Self {
            about: About { version },
            database: Database::new(pool),
            db_support,
            filesystem: Filesystem::new(home),
            archive: archive.clone(),
        })
    }

    pub async fn prepare(&self) -> result::Result<(), String> {
        self.db_support.check_schema_version().await?;

        Ok(())
    }

    pub async fn archive(&self) -> result::Result<(), String> {
        let archive = self
            .archive
            .as_deref()
            .ok_or_else(|| String::from("no archive location specified"))?
            .join(DATABASE_DUMP_FILENAME);

        let dump = archive.join(DATABASE_DUMP_FILENAME);
        self.db_support.dump(&dump).await?;

        Ok(())
    }

    pub async fn init(&self) -> result::Result<(), String> {
        self.db_support.init().await
    }

    pub async fn migrate(&self) -> result::Result<(), String> {
        self.db_support.migrate().await
    }

    pub async fn reset(&self) -> result::Result<(), String> {
        self.db_support.reset().await
    }

    pub async fn restore(&self, path: &Path) -> result::Result<(), String> {
        self.db_support.restore(path).await
    }

    pub fn about(&self) -> &About {
        &self.about
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
            .fetch_buckets_all()
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

    pub async fn get_object(&self, object_id: &Uuid) -> Result<File> {
        self.filesystem.object(object_id).await
    }

    pub async fn get_object_metadata(
        &self,
        bucket_id: &Uuid,
        object_id: &Uuid,
    ) -> Result<Object> {
        self.database
            .get_object(bucket_id, object_id)
            .await?
            .map(|object| object.into())
            .ok_or_not_found("Object")
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

        self.filesystem.part(id).await
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

        info!(
            "Pruned {} object{}",
            objects.len(),
            match objects.len() {
                1 => "",
                _ => "s",
            }
        );

        Ok(objects.into_iter().map(|object| object.into()).collect())
    }

    pub async fn remove_bucket(&self, bucket_id: &Uuid) -> Result<()> {
        Ok(self.database.remove_bucket(bucket_id).await?)
    }

    pub async fn remove_object(
        &self,
        bucket_id: &Uuid,
        object_id: &Uuid,
    ) -> Result<Object> {
        self.database
            .remove_object(bucket_id, object_id)
            .await?
            .map(|object| object.into())
            .ok_or_not_found("Bucket or object not found")
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

    pub async fn shutdown(&self) {
        self.database.close().await
    }
}
