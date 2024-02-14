#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("SQL Error")]
    Sql(#[from] sqlx::Error),

    #[error("This object is being written to by another request")]
    WriteLock,

    #[error("{0}")]
    Internal(String),

    #[error("task already in progress")]
    InProgress,

    #[error("{0} not found")]
    NotFound(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait OptionNotFound {
    type Value;

    fn ok_or_not_found(self, entity: &'static str) -> Result<Self::Value>;
}

impl<T> OptionNotFound for Option<T> {
    type Value = T;

    fn ok_or_not_found(self, entity: &'static str) -> Result<Self::Value> {
        match self {
            Some(value) => Ok(value),
            None => Err(Error::NotFound(entity)),
        }
    }
}

macro_rules! internal {
    ($msg:literal) => {
        return Err(crate::error::Error::Internal($msg.into()))
    };
    ($fmt:expr, $($args:expr),+) => {
        return Err(crate::error::Error::Internal(format!($fmt, $($args),*)))
    };
}

pub(crate) use internal;
