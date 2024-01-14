use crate::conf::Config;

use fstore_core::{Database, Filesystem, ObjectStore, Version};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use url::Url;

pub async fn start(
    version: Version,
    config: &Config,
) -> Result<Arc<ObjectStore>, String> {
    let url =
        Url::parse_with_params("postgresql://", &config.database.connection)
            .map_err(|err| {
                format!("Invalid database connection parameters: {err}")
            })?;

    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(url.as_str())
        .await
        .map_err(|err| {
            format!("Failed to establish database connection: {err}")
        })?;

    let db = Database::new(pool);
    let fs = Filesystem::new(&config.home);

    Ok(Arc::new(ObjectStore::new(version, db, fs)))
}
