use crate::server::error::Result;
use crate::server::AppState;

use axum::{
    async_trait,
    body::Bytes,
    extract::{rejection::BytesRejection, FromRequest, Path, Request, State},
    http::{
        header::{CONTENT_LENGTH, CONTENT_TYPE},
        StatusCode,
    },
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_extra::{body::AsyncReadBody, headers::ContentLength, TypedHeader};
use fstore::{Bucket, Object, ObjectError, RemoveResult, StoreTotals};
use fstore_core::About;
use mime::Mime;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug)]
struct IdList(Vec<Uuid>);

#[derive(Debug)]
enum IdListRejection {
    BytesRejection(BytesRejection),
    InvalidUtf8(std::str::Utf8Error),
    InvalidUuid(uuid::Error),
    MissingIdListContentType(String),
}

impl IntoResponse for IdListRejection {
    fn into_response(self) -> Response {
        match self {
            Self::BytesRejection(err) => (
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {err}"),
            ),
            Self::InvalidUtf8(err) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid UTF-8 in request body: {err}"),
            ),
            Self::InvalidUuid(err) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid UUID in request body: {err}"),
            ),
            Self::MissingIdListContentType(err) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                format!(
                    "Expected content of type `{}`: {err}",
                    mime::TEXT_PLAIN_UTF_8
                ),
            ),
        }
        .into_response()
    }
}

impl From<BytesRejection> for IdListRejection {
    fn from(value: BytesRejection) -> Self {
        Self::BytesRejection(value)
    }
}

#[async_trait]
impl<S> FromRequest<S> for IdList
where
    S: Send + Sync,
{
    type Rejection = IdListRejection;

    async fn from_request(
        req: Request,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let content_type = match req.headers().get(CONTENT_TYPE) {
            Some(content_type) => match content_type.to_str() {
                Ok(content_type) => match content_type.parse::<Mime>() {
                    Ok(mime) => mime,
                    Err(err) => {
                        return Err(IdListRejection::MissingIdListContentType(
                            format!(
                                "failed to parse provided content type: {err}"
                            ),
                        ))
                    }
                },
                Err(err) => {
                    return Err(IdListRejection::MissingIdListContentType(
                        format!("{err}"),
                    ))
                }
            },
            None => {
                return Err(IdListRejection::MissingIdListContentType(
                    "no content type provided".into(),
                ))
            }
        };

        if content_type != mime::TEXT_PLAIN_UTF_8 {
            return Err(IdListRejection::MissingIdListContentType(format!(
                "received '{content_type}'"
            )));
        }

        let bytes = Bytes::from_request(req, state).await?;

        let ids: std::result::Result<Vec<_>, _> = std::str::from_utf8(&bytes)
            .map_err(IdListRejection::InvalidUtf8)?
            .lines()
            .map(Uuid::parse_str)
            .collect();

        match ids {
            Ok(ids) => Ok(IdList(ids)),
            Err(err) => Err(IdListRejection::InvalidUuid(err)),
        }
    }
}

#[derive(Debug, Serialize)]
struct NewPart {
    id: Uuid,
    written: u64,
}

async fn about(State(AppState { store }): State<AppState>) -> Json<About> {
    Json(*store.about())
}

async fn add_bucket(
    State(AppState { store }): State<AppState>,
    Path(bucket): Path<String>,
) -> Result<Json<Bucket>> {
    Ok(Json(store.add_bucket(&bucket).await?))
}

async fn add_object(
    State(AppState { store }): State<AppState>,
    Path(bucket): Path<Uuid>,
    request: Request,
) -> Result<Json<Object>> {
    let mut part = store.get_part(None).await?;

    part.stream_to_file(request.into_body().into_data_stream())
        .await?;

    let object = store.commit_part(&bucket, part.id()).await?;

    Ok(Json(object))
}

async fn append_part(
    State(AppState { store }): State<AppState>,
    Path(id): Path<Uuid>,
    request: Request,
) -> Result<String> {
    let mut part = store.get_part(Some(&id)).await?;

    let bytes = part
        .stream_to_file(request.into_body().into_data_stream())
        .await?;

    Ok(bytes.to_string())
}

async fn commit_part(
    State(AppState { store }): State<AppState>,
    Path((bucket, id)): Path<(String, Uuid)>,
    content_length: Option<TypedHeader<ContentLength>>,
    request: Request,
) -> Result<Json<Object>> {
    let bucket = store.get_bucket(&bucket).await?;

    if let Some(TypedHeader(ContentLength(_content_length))) = content_length {
        let mut part = store.get_part(Some(&id)).await?;
        part.stream_to_file(request.into_body().into_data_stream())
            .await?;
    }

    let object = store.commit_part(&bucket.id, &id).await?;

    Ok(Json(object))
}

async fn get_bucket(
    State(AppState { store }): State<AppState>,
    Path(bucket): Path<String>,
) -> Result<Json<Bucket>> {
    Ok(Json(store.get_bucket(&bucket).await?))
}

async fn get_buckets(
    State(AppState { store }): State<AppState>,
) -> Result<Json<Vec<Bucket>>> {
    Ok(Json(store.get_buckets().await?))
}

async fn get_object_data(
    State(AppState { store }): State<AppState>,
    Path((bucket, object)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    let object = store.get_object_metadata(&bucket, &object).await?;
    let file = store.get_object(&object.id).await?;

    let headers = [
        (CONTENT_LENGTH, object.size.to_string()),
        (CONTENT_TYPE, object.media_type()),
    ];
    let body = AsyncReadBody::new(file);

    Ok((headers, body).into_response())
}

async fn get_object_errors(
    State(AppState { store }): State<AppState>,
) -> Result<Json<Vec<ObjectError>>> {
    Ok(Json(store.get_object_errors().await?))
}

async fn get_object_metadata(
    State(AppState { store }): State<AppState>,
    Path((bucket_id, object_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Object>> {
    Ok(Json(
        store.get_object_metadata(&bucket_id, &object_id).await?,
    ))
}

async fn new_part(
    State(AppState { store }): State<AppState>,
    request: Request,
) -> Result<Json<NewPart>> {
    let mut part = store.get_part(None).await?;

    let bytes = part
        .stream_to_file(request.into_body().into_data_stream())
        .await?;

    Ok(Json(NewPart {
        id: *part.id(),
        written: bytes,
    }))
}

async fn prune(
    State(AppState { store }): State<AppState>,
) -> Result<Json<Vec<Object>>> {
    Ok(Json(store.prune().await?))
}

async fn remove_bucket(
    State(AppState { store }): State<AppState>,
    Path(bucket): Path<Uuid>,
) -> Result<StatusCode> {
    store.remove_bucket(&bucket).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_object(
    State(AppState { store }): State<AppState>,
    Path((bucket, object)): Path<(Uuid, Uuid)>,
) -> Result<Json<Object>> {
    Ok(Json(store.remove_object(&bucket, &object).await?))
}

async fn remove_objects(
    State(AppState { store }): State<AppState>,
    Path(bucket): Path<Uuid>,
    IdList(objects): IdList,
) -> Result<Json<RemoveResult>> {
    Ok(Json(store.remove_objects(&bucket, &objects).await?))
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
            "/bucket/:bucket",
            get(get_bucket)
                .put(add_bucket)
                .post(add_object)
                .delete(remove_bucket),
        )
        .route("/bucket/:name/objects", delete(remove_objects))
        .route("/bucket/:old/:new", put(rename_bucket))
        .route("/buckets", get(get_buckets))
        .route("/object", post(new_part))
        .route("/object/:id", post(append_part))
        .route(
            "/object/:bucket/:id",
            get(get_object_metadata)
                .put(commit_part)
                .delete(remove_object),
        )
        .route("/object/:bucket/:object/data", get(get_object_data))
        .route("/object/errors", get(get_object_errors))
        .route("/objects", delete(prune))
        .route("/status", get(status))
}
