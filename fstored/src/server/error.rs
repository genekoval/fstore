use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use log::error;

pub struct Error(fstore_core::Error);

impl From<fstore_core::Error> for Error {
    fn from(value: fstore_core::Error) -> Self {
        Self(value)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        error!("{}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong")
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
