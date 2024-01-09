use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Internal server error")]
    Internal,
}

pub type Result<T> = std::result::Result<T, Error>;
