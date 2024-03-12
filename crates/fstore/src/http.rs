use crate::{
    error::{Error, ErrorKind, Result},
    model, About, Object, ObjectError, ObjectSummary, RemoveResult,
    StoreTotals,
};

use bytes::Bytes;
use futures_core::{Stream, TryStream};
use mime::{Mime, TEXT_PLAIN_UTF_8};
use reqwest::{
    header::CONTENT_TYPE, Body, RequestBuilder, Response, StatusCode, Url,
};
use std::{error, fmt::Write};
use tokio::io::AsyncRead;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

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

    pub async fn add_object_stream<S>(
        &self,
        bucket: Uuid,
        stream: S,
    ) -> Result<Object>
    where
        S: TryStream + Send + 'static,
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

    pub async fn stream_object(
        &self,
        bucket: Uuid,
        object: Uuid,
    ) -> Result<(ObjectSummary, impl Stream<Item = std::io::Result<Bytes>>)>
    {
        let res = self
            .client
            .get(self.path(&[
                "object",
                &bucket.to_string(),
                &object.to_string(),
                "data",
            ]))
            .send_and_check()
            .await?;

        let content_length: u64 = res
            .headers()
            .get("content-length")
            .expect("server response should contain a content-length header")
            .to_str()
            .expect("content-length header value should be valid ASCII")
            .parse()
            .expect("content-length header value should be a valid number");

        let content_type: String = res
            .headers()
            .get("content-type")
            .expect("cerver response should contain a content-type header")
            .to_str()
            .expect("content-type header value should be valid ASCII")
            .into();

        let stream = res
            .bytes_stream()
            .map(|result| result.map_err(std::io::Error::other));

        Ok((
            ObjectSummary {
                media_type: content_type,
                size: content_length,
            },
            stream,
        ))
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

    pub async fn add_object_stream<S>(&self, stream: S) -> Result<Object>
    where
        S: TryStream + Send + 'static,
        S::Error: Into<Box<dyn error::Error + Send + Sync>>,
        Bytes: From<S::Ok>,
    {
        self.client.add_object_stream(self.id, stream).await
    }

    pub async fn get_object(&self, id: Uuid) -> Result<Object> {
        self.client.get_object(self.id, id).await
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

    pub async fn stream_object(
        &self,
        id: Uuid,
    ) -> Result<(ObjectSummary, impl Stream<Item = std::io::Result<Bytes>>)>
    {
        self.client.stream_object(self.id, id).await
    }
}
