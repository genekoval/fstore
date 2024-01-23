use crate::server::error::Result;
use crate::server::AppState;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use fstore::{Bucket, StoreTotals};
use fstore_core::About;
use uuid::Uuid;

async fn about(State(AppState { store }): State<AppState>) -> Json<About> {
    Json(*store.about())
}

async fn add_bucket(
    State(AppState { store }): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Bucket>> {
    Ok(Json(store.add_bucket(&name).await?))
}

async fn get_bucket(
    State(AppState { store }): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Bucket>> {
    Ok(Json(store.get_bucket(&name).await?))
}

async fn get_buckets(
    State(AppState { store }): State<AppState>,
) -> Result<Json<Vec<Bucket>>> {
    Ok(Json(store.get_buckets().await?))
}

async fn remove_bucket(
    State(AppState { store }): State<AppState>,
    Path(name): Path<Uuid>,
) -> Result<StatusCode> {
    store.remove_bucket(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn rename_bucket(
    State(AppState { store }): State<AppState>,
    Path((old, new)): Path<(Uuid, String)>,
) -> Result<StatusCode> {
    store.rename_bucket(&old, &new).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn status(
    State(AppState { store }): State<AppState>,
) -> Result<Json<StoreTotals>> {
    Ok(Json(store.get_totals().await?))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(about))
        .route(
            "/bucket/:name",
            get(get_bucket).put(add_bucket).delete(remove_bucket),
        )
        .route("/bucket/:old/:new", put(rename_bucket))
        .route("/buckets", get(get_buckets))
        .route("/status", get(status))
}
