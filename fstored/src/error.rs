#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("SQL Error")]
    SqlError(#[from] sqlx::Error),

    #[error("This object is being written to by another request")]
    WriteLock,

    #[error("{0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! internal {
    ($msg:literal) => {
        return Err(crate::error::Error::Internal($msg.into()))
    };
    ($fmt:expr, $($args:expr),+) => {
        return Err(crate::error::Error::Internal(format!($fmt, $($args),*)))
    };
}

pub(crate) use internal;
