mod error;
mod router;

use crate::{conf::Http, Result};

use fstore_core::ObjectStore;
use log::{error, info};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    store: Arc<ObjectStore>,
}

pub async fn serve(
    config: &Http,
    store: Arc<ObjectStore>,
    parent: &mut dmon::Parent,
) -> Result {
    info!(
        "fstore version {} starting up",
        store.about().version.number
    );

    store.prepare().await?;

    let app = router::routes().with_state(AppState { store });

    axum_unix::serve(&config.listen, app, |_| {
        if let Err(err) = parent.notify() {
            error!(
                "Failed to notify parent process of successful start: {err}"
            );
        }
    })
    .await?;

    info!("Server shutting down");
    Ok(())
}