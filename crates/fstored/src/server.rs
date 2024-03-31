mod error;
mod router;

use crate::{conf::Http, Result};

use axum_unix::shutdown_signal;
use fstore_core::ObjectStore;
use log::{error, info};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

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
    let token = CancellationToken::new();

    let mut handles = Vec::new();

    for endpoint in &config.listen {
        let handle =
            axum_unix::serve(endpoint, app.clone(), token.clone(), |_| {
                if let Err(err) = parent.notify() {
                    error!(
                        "Failed to notify parent process of \
                        successful start: {err}"
                    );
                }
            })
            .await;

        match handle {
            Ok(handle) => handles.push(handle),
            Err(err) => error!("{err}"),
        }
    }

    if handles.is_empty() {
        return Err("No servers could be started".into());
    }

    shutdown_signal().await;
    token.cancel();
    info!("Server shutting down");

    for handle in handles {
        if let Err(err) = handle.await {
            error!("Failed to join server task: {err}");
        }
    }

    Ok(())
}
