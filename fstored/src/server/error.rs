use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use log::error;
use sqlx::error::Error as SqlError;

pub struct Error(fstore_core::Error);

impl From<fstore_core::Error> for Error {
    fn from(value: fstore_core::Error) -> Self {
        Self(value)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        use fstore_core::Error::*;

        let error = self.0;

        match &error {
            Sql(sql) => match sql {
                SqlError::RowNotFound => {
                    return (StatusCode::NOT_FOUND, "Not found").into_response()
                }
                _ => error!("{error}: {sql}"),
            },
            _ => error!("{error}"),
        };

        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong")
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
