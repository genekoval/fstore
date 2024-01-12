use crate::conf::Config;

use fstore_core::{Database, Filesystem, ObjectStore};
use sqlx::postgres::PgPoolOptions;
use url::Url;

pub async fn start(config: &Config) -> Result<ObjectStore, String> {
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

    Ok(ObjectStore::new(db, fs))
}
