use crate::server::error::Result;
use crate::server::AppState;

use axum::{extract::State, routing::get, Json, Router};
use fstore::StoreTotals;
use fstore_core::About;

async fn about(State(AppState { store }): State<AppState>) -> Json<About> {
    Json(*store.about())
}

async fn status(
    State(AppState { store }): State<AppState>,
) -> Result<Json<StoreTotals>> {
    Ok(Json(store.get_totals().await?))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(about))
        .route("/status", get(status))
}
