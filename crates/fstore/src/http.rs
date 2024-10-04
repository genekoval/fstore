#[cfg(feature = "axum")]
mod axum;

use crate::{
    error::{Error, ErrorKind, Result},
    model, About, Object, ObjectError, RemoveResult, StoreTotals,
};

pub use headers::Range;

use bytes::Bytes;
use futures_core::{Stream, TryStream};
use headers::HeaderMapExt;
use mime::{Mime, TEXT_PLAIN_UTF_8};
use reqwest::{
    header::{HeaderMap, CONTENT_TYPE},
    Body, Method, RequestBuilder, Response, StatusCode, Url,
};
use std::{
    error,
    fmt::{self, Display, Write},
    ops::{Bound, RangeBounds},
};
use tokio::io::AsyncRead;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub enum ProxyMethod {
    Get,
    Head,
}

impl Display for ProxyMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => f.write_str("GET"),
            Self::Head => f.write_str("HEAD"),
        }
    }
}

#[derive(Debug)]
pub struct ProxyResponse<S> {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub stream: S,
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::other(error.to_string())
    }
}

trait RequestExt {
    fn content_type(self, mime: Mime) -> Self;

    async fn send_and_check(self) -> Result<Response>;
}

impl RequestExt for RequestBuilder {
    fn content_type(self, mime: Mime) -> Self {
        self.header(CONTENT_TYPE, mime.as_ref())
    }

    async fn send_and_check(self) -> Result<Response> {
        let response = self
            .send()
            .await
            .map_err(|err| Error::other(format!("Request failed: {err}")))?;

        let status = response.status();

        if status.is_success() {
            return Ok(response);
        }

        let kind = if status == StatusCode::NOT_FOUND {
            ErrorKind::NotFound
        } else if status.is_client_error() {
            ErrorKind::Client
        } else if status.is_server_error() {
            ErrorKind::Server
        } else {
            ErrorKind::Other
        };

        match response.text().await {
            Ok(text) => Err(Error::new(kind, text)),
            Err(err) => Err(Error::other(format!(
                "failed to read response body: {err}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    client: reqwest::Client,
    url: Url,
}

impl Client {
    pub fn new(url: &url::Url) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.clone(),
        }
    }

    pub fn url(&self) -> String {
        self.url.to_string()
    }

    pub async fn about(&self) -> Result<About> {
        Ok(self
            .client
            .get(self.url.clone())
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn add_bucket(&self, name: &str) -> Result<model::Bucket> {
        Ok(self
            .client
            .put(self.path(&["bucket", name]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn add_object<T>(&self, bucket: Uuid, object: T) -> Result<Object>
    where
        T: AsyncRead + Send + Sync + 'static,
    {
        let stream = ReaderStream::new(object);
        self.add_object_stream(bucket, stream).await
    }

    pub async fn add_object_bytes(
        &self,
        bucket: Uuid,
        object: Bytes,
    ) -> Result<Object> {
        Ok(self
            .client
            .post(self.path(&["bucket", &bucket.to_string()]))
            .body(object)
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn add_object_stream<S>(
        &self,
        bucket: Uuid,
        stream: S,
    ) -> Result<Object>
    where
        S: TryStream + Send + Sync + 'static,
        S::Error: Into<Box<dyn error::Error + Send + Sync>>,
        Bytes: From<S::Ok>,
    {
        Ok(self
            .client
            .post(self.path(&["bucket", &bucket.to_string()]))
            .body(Body::wrap_stream(stream))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub fn bucket(self, id: &Uuid) -> Bucket {
        Bucket::new(self, id)
    }

    pub async fn clone_bucket(
        &self,
        original: Uuid,
        name: &str,
    ) -> Result<model::Bucket> {
        Ok(self
            .client
            .post(self.path(&["bucket", &original.to_string(), name]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn get_all_objects(
        &self,
        bucket_id: Uuid,
    ) -> Result<Vec<Object>> {
        Ok(self
            .client
            .get(self.path(&["object", &bucket_id.to_string(), "all"]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn get_bucket(
        &self,
        name: &str,
    ) -> Result<(Bucket, model::Bucket)> {
        let url = self.path(&["bucket", name]);

        let bucket: model::Bucket =
            self.client.get(url).send_and_check().await?.json().await?;

        Ok((Bucket::new(self.clone(), &bucket.id), bucket))
    }

    pub async fn get_buckets(&self) -> Result<Vec<model::Bucket>> {
        Ok(self
            .client
            .get(self.path(&["buckets"]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn get_object(
        &self,
        bucket: Uuid,
        object: Uuid,
    ) -> Result<Object> {
        Ok(self
            .client
            .get(self.path(&[
                "object",
                &bucket.to_string(),
                &object.to_string(),
            ]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn get_objects(
        &self,
        bucket: Uuid,
        objects: &[Uuid],
    ) -> Result<Vec<Object>> {
        if objects.is_empty() {
            return Ok(Default::default());
        }

        let body = objects
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(self
            .client
            .get(self.path(&["object", &bucket.to_string()]))
            .content_type(TEXT_PLAIN_UTF_8)
            .body(body)
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    async fn get_object_data(
        &self,
        bucket: Uuid,
        object: Uuid,
        bounds: Option<(Bound<&u64>, Bound<&u64>)>,
    ) -> Result<Response> {
        let mut builder = self.client.get(self.path(&[
            "object",
            &bucket.to_string(),
            &object.to_string(),
            "data",
        ]));

        if let Some(bounds) = bounds {
            let range = Range::bytes(bounds)
                .map_err(|err| Error::other(err.to_string()))?;

            let mut headers = HeaderMap::new();
            headers.typed_insert(range);

            builder = builder.headers(headers);
        }

        builder.send_and_check().await
    }

    pub async fn get_object_bytes(
        &self,
        bucket: Uuid,
        object: Uuid,
    ) -> Result<Bytes> {
        Ok(self
            .get_object_data(bucket, object, None)
            .await?
            .bytes()
            .await?)
    }

    pub async fn get_object_bytes_range(
        &self,
        bucket: Uuid,
        object: Uuid,
        range: impl RangeBounds<u64>,
    ) -> Result<Bytes> {
        let range = Some((range.start_bound(), range.end_bound()));
        Ok(self
            .get_object_data(bucket, object, range)
            .await?
            .bytes()
            .await?)
    }

    pub async fn get_object_stream(
        &self,
        bucket: Uuid,
        object: Uuid,
    ) -> Result<impl Stream<Item = std::io::Result<Bytes>>> {
        Ok(self
            .get_object_data(bucket, object, None)
            .await?
            .bytes_stream()
            .map(|result| result.map_err(std::io::Error::other)))
    }

    pub async fn get_object_stream_range(
        &self,
        bucket: Uuid,
        object: Uuid,
        range: impl RangeBounds<u64>,
    ) -> Result<impl Stream<Item = std::io::Result<Bytes>>> {
        let range = Some((range.start_bound(), range.end_bound()));
        Ok(self
            .get_object_data(bucket, object, range)
            .await?
            .bytes_stream()
            .map(|result| result.map_err(std::io::Error::other)))
    }

    pub async fn get_object_errors(&self) -> Result<Vec<ObjectError>> {
        Ok(self
            .client
            .get(self.path(&["object", "errors"]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    fn path<I>(&self, segments: I) -> Url
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.url.clone();
        url.path_segments_mut().unwrap().extend(segments);
        url
    }

    pub async fn proxy(
        &self,
        bucket: Uuid,
        object: Uuid,
        method: ProxyMethod,
        range: Option<Range>,
    ) -> reqwest::Result<
        ProxyResponse<impl Stream<Item = std::io::Result<Bytes>>>,
    > {
        let method = match method {
            ProxyMethod::Get => Method::GET,
            ProxyMethod::Head => Method::HEAD,
        };

        let url = self.path(&[
            "object",
            &bucket.to_string(),
            &object.to_string(),
            "data",
        ]);

        let mut headers = HeaderMap::new();

        if let Some(range) = range {
            headers.typed_insert(range);
        }

        let response = self
            .client
            .request(method, url)
            .headers(headers)
            .send()
            .await?;

        let status = response.status();
        let mut headers = HeaderMap::new();

        for name in [
            "accept-ranges",
            "content-length",
            "content-range",
            "content-type",
        ] {
            if let Some(value) = response.headers().get(name) {
                headers.insert(name, value.clone());
            }
        }

        let stream = response
            .bytes_stream()
            .map(|result| result.map_err(std::io::Error::other));

        Ok(ProxyResponse {
            status,
            headers,
            stream,
        })
    }

    pub async fn prune(&self) -> Result<Vec<Object>> {
        Ok(self
            .client
            .delete(self.path(&["objects"]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn remove_bucket(&self, id: &Uuid) -> Result<()> {
        self.client
            .delete(self.path(&["bucket", &id.to_string()]))
            .send_and_check()
            .await?;

        Ok(())
    }

    pub async fn remove_object(
        &self,
        bucket: Uuid,
        object: Uuid,
    ) -> Result<Object> {
        Ok(self
            .client
            .delete(self.path(&[
                "object",
                &bucket.to_string(),
                &object.to_string(),
            ]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn remove_objects(
        &self,
        bucket: Uuid,
        objects: &[Uuid],
    ) -> Result<RemoveResult> {
        if objects.is_empty() {
            return Ok(Default::default());
        }

        let mut body = String::new();
        objects
            .iter()
            .for_each(|id| writeln!(body, "{id}").unwrap());

        Ok(self
            .client
            .delete(self.path(&["bucket", &bucket.to_string(), "objects"]))
            .content_type(TEXT_PLAIN_UTF_8)
            .body(body)
            .send_and_check()
            .await?
            .json()
            .await?)
    }

    pub async fn rename_bucket(&self, old: &Uuid, new: &str) -> Result<()> {
        let mut url = self.url.clone();
        url.path_segments_mut().unwrap().extend(&[
            "bucket",
            &old.to_string(),
            new,
        ]);

        self.client.put(url).send_and_check().await?;

        Ok(())
    }

    pub async fn status(&self) -> Result<StoreTotals> {
        Ok(self
            .client
            .get(self.path(&["status"]))
            .send_and_check()
            .await?
            .json()
            .await?)
    }
}

#[derive(Clone, Debug)]
pub struct Bucket {
    client: Client,
    id: Uuid,
}

impl Bucket {
    fn new(client: Client, id: &Uuid) -> Self {
        Self { client, id: *id }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub async fn add_object<T>(&self, object: T) -> Result<Object>
    where
        T: AsyncRead + Send + Sync + 'static,
    {
        self.client.add_object(self.id, object).await
    }

    pub async fn add_object_bytes(&self, object: Bytes) -> Result<Object> {
        self.client.add_object_bytes(self.id, object).await
    }

    pub async fn add_object_stream<S>(&self, stream: S) -> Result<Object>
    where
        S: TryStream + Send + Sync + 'static,
        S::Error: Into<Box<dyn error::Error + Send + Sync>>,
        Bytes: From<S::Ok>,
    {
        self.client.add_object_stream(self.id, stream).await
    }

    pub async fn clone_as(&self, name: &str) -> Result<Self> {
        let clone = self.client.clone_bucket(self.id, name).await?;

        Ok(Self {
            client: self.client.clone(),
            id: clone.id,
        })
    }

    pub async fn get_all_objects(&self) -> Result<Vec<Object>> {
        self.client.get_all_objects(self.id).await
    }

    pub async fn get_object(&self, id: Uuid) -> Result<Object> {
        self.client.get_object(self.id, id).await
    }

    pub async fn get_objects(&self, objects: &[Uuid]) -> Result<Vec<Object>> {
        self.client.get_objects(self.id, objects).await
    }

    pub async fn get_object_bytes(&self, id: Uuid) -> Result<Bytes> {
        self.client.get_object_bytes(self.id, id).await
    }

    pub async fn get_object_bytes_range(
        &self,
        id: Uuid,
        range: impl RangeBounds<u64>,
    ) -> Result<Bytes> {
        self.client.get_object_bytes_range(self.id, id, range).await
    }

    pub async fn get_object_stream(
        &self,
        id: Uuid,
    ) -> Result<impl Stream<Item = std::io::Result<Bytes>>> {
        self.client.get_object_stream(self.id, id).await
    }

    pub async fn get_object_stream_range(
        &self,
        id: Uuid,
        range: impl RangeBounds<u64>,
    ) -> Result<impl Stream<Item = std::io::Result<Bytes>>> {
        self.client
            .get_object_stream_range(self.id, id, range)
            .await
    }

    pub async fn proxy(
        &self,
        object: Uuid,
        method: ProxyMethod,
        range: Option<Range>,
    ) -> reqwest::Result<
        ProxyResponse<impl Stream<Item = std::io::Result<Bytes>>>,
    > {
        self.client.proxy(self.id, object, method, range).await
    }

    pub async fn remove_object(&self, id: Uuid) -> Result<Object> {
        self.client.remove_object(self.id, id).await
    }

    pub async fn remove_objects(
        &self,
        objects: &[Uuid],
    ) -> Result<RemoveResult> {
        self.client.remove_objects(self.id, objects).await
    }

    pub async fn rename(&self, name: &str) -> Result<()> {
        self.client.rename_bucket(&self.id, name).await
    }
}
