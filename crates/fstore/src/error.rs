use std::fmt::{self, Display, Formatter};

#[derive(Copy, Clone, Debug)]
pub enum ErrorKind {
    Client,
    NotFound,
    Server,
    Other,
}

#[derive(Debug)]
pub struct Error {
    message: String,
    kind: ErrorKind,
}

impl Error {
    pub fn new(kind: ErrorKind, message: String) -> Self {
        Self { message, kind }
    }

    pub fn other(message: String) -> Self {
        Self::new(ErrorKind::Other, message)
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
