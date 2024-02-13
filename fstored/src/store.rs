use crate::{conf::Config, Result};

use fstore_core::{ObjectStore, StoreOptions, Version};
use std::{future::Future, sync::Arc};

pub async fn start<F, Fut>(
    version: Version,
    Config {
        database,
        home,
        archive,
        ..
    }: &Config,
    f: F,
) -> Result
where
    F: FnOnce(Arc<ObjectStore>) -> Fut,
    Fut: Future<Output = Result>,
{
    let options = StoreOptions {
        version,
        database,
        home: home.as_path(),
        archive,
    };

    let store = Arc::new(ObjectStore::new(options).await?);

    let result = f(store.clone()).await;

    store.shutdown().await;

    result
}
