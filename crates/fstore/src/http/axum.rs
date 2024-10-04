use super::ProxyResponse;

use axum::response::{IntoResponse, Response};
use axum_extra::body::AsyncReadBody;
use bytes::Bytes;
use futures_core::Stream;
use std::io;
use tokio_util::io::StreamReader;

impl<S> IntoResponse for ProxyResponse<S>
where
    S: Stream<Item = io::Result<Bytes>> + Send + Sync + 'static,
{
    fn into_response(self) -> Response {
        let reader = StreamReader::new(self.stream);
        let body = AsyncReadBody::new(reader);

        (self.status, self.headers, body).into_response()
    }
}
