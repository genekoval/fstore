mod error;
mod router;

use crate::conf::Http;

use fstore_core::ObjectStore;
use log::{error, info};
use std::sync::Arc;
use tokio::{net::TcpListener, signal};

#[derive(Clone)]
struct AppState {
    store: Arc<ObjectStore>,
}

pub async fn serve(
    config: &Http,
    store: Arc<ObjectStore>,
    parent: &mut dmon::Parent,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "fstore version {} starting up",
        store.about().version.number
    );

    let app = router::routes().with_state(AppState { store });
    let listener = TcpListener::bind(&config.listen).await?;

    if let Err(err) = parent.notify() {
        error!("Failed to notify parent process of successful start: {err}");
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutting down");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
