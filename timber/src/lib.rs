mod file;
mod serde;
pub mod syslog;

use log::{LevelFilter, Log, Metadata, SetLoggerError};
use std::{io, path::PathBuf};

struct Logger {
    filter: LevelFilter,
}

impl Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.filter
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },

    #[error(transparent)]
    SetLogger(#[from] SetLoggerError),
}

#[derive(Clone, Default, Debug)]
pub enum Sink {
    #[default]
    Stderr,
    Stdout,
    File(PathBuf),
    Syslog(syslog::Config),
}

pub struct Config {
    pub level: LevelFilter,
    pub sink: Sink,
}

impl Config {
    pub fn init(self) -> Result<(), Error> {
        use file::{File, Stderr, Stdout};
        use syslog::Syslog;

        let logger = Logger { filter: self.level };

        let logger: Box<dyn Log> = match self.sink {
            Sink::Stderr => Box::new(Stderr::new(logger)),
            Sink::Stdout => Box::new(Stdout::new(logger)),
            Sink::File(path) => Box::new(File::new(logger, &path)?),
            Sink::Syslog(config) => Box::new(Syslog::new(logger, &config)),
        };

        Ok(log::set_boxed_logger(logger)
            .map(|()| log::set_max_level(self.level))?)
    }

    pub fn max_level(mut self, filter: LevelFilter) -> Self {
        self.level = filter;
        self
    }

    pub fn sink(mut self, sink: Sink) -> Self {
        self.sink = sink;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            level: LevelFilter::max(),
            sink: Default::default(),
        }
    }
}

pub fn new() -> Config {
    Default::default()
}
