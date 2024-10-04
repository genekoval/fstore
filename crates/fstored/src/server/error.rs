use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_range::RangeNotSatisfiable;
use log::error;
use sqlx::error::Error as SqlError;

pub enum Error {
    Core(fstore_core::Error),
    RangeNotSatisfiable(RangeNotSatisfiable),
}

impl From<fstore_core::Error> for Error {
    fn from(value: fstore_core::Error) -> Self {
        Self::Core(value)
    }
}

impl From<RangeNotSatisfiable> for Error {
    fn from(value: RangeNotSatisfiable) -> Self {
        Self::RangeNotSatisfiable(value)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        use fstore_core::Error::*;

        if let Self::Core(error) = &self {
            match error {
                Sql(sql) => match sql {
                    SqlError::RowNotFound => {
                        return (StatusCode::NOT_FOUND, "Not found")
                            .into_response()
                    }
                    error => error!("{error}: {sql}"),
                },
                NotFound(_) => {
                    return (StatusCode::NOT_FOUND, format!("{error}"))
                        .into_response()
                }
                _ => error!("{error}"),
            }
        } else if let Self::RangeNotSatisfiable(error) = self {
            return error.into_response();
        }

        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong")
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
