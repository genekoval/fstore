mod error;
mod router;

use crate::conf::Http;

use fstore_core::ObjectStore;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    store: Arc<ObjectStore>,
}

pub async fn serve(
    config: &Http,
    store: Arc<ObjectStore>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router::routes().with_state(AppState { store });

    let listener = TcpListener::bind(&config.listen).await?;
    Ok(axum::serve(listener, app).await?)
}
