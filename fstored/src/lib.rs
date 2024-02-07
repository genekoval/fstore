pub mod conf;
pub mod server;
pub mod store;

pub use conf::Config;
pub use fstore_core::ObjectStore;

use std::{error::Error, result};

pub type BoxError = Box<dyn Error + Send + Sync + 'static>;
pub type Result = result::Result<(), BoxError>;
