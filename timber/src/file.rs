use log::{Level, Log, Record};

use crate::{Error, Logger};

use chrono::Local;
use owo_colors::{colors::css::*, OwoColorize, Style};
use std::{
    fs::{self, OpenOptions},
    io,
    ops::DerefMut,
    path::Path,
    sync::Mutex,
};

fn write_record<Writer>(writer: &mut Writer, record: &Record)
where
    Writer: std::io::Write,
{
    let timestamp = Local::now().format("%b %d %Y %I:%M:%S%.6f %p");
    let timestamp = timestamp.fg::<DimGray>();

    let level = record.level();
    let level = level.style(match level {
        Level::Error => Style::new().fg::<OrangeRed>(),
        Level::Warn => Style::new().fg::<Yellow>(),
        Level::Info => Style::new().fg::<SteelBlue>(),
        Level::Debug => Style::new().fg::<Plum>(),
        Level::Trace => Style::new().fg::<BlueViolet>(),
    });

    let target = record.target();
    let target = target.fg::<MediumSeaGreen>();

    let message = record.args();

    writeln!(writer, "{timestamp} {level:<5} {target} {message}").unwrap();
}

pub struct Stderr {
    inner: Logger,
    handle: io::Stderr,
}

impl Stderr {
    pub fn new(logger: Logger) -> Self {
        Self {
            inner: logger,
            handle: io::stderr(),
        }
    }
}

impl Log for Stderr {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            write_record(&mut self.handle.lock(), record);
        }
    }

    fn flush(&self) {}
}

pub struct Stdout {
    inner: Logger,
    handle: io::Stdout,
}

impl Stdout {
    pub fn new(logger: Logger) -> Self {
        Self {
            inner: logger,
            handle: io::stdout(),
        }
    }
}

impl Log for Stdout {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            write_record(&mut self.handle.lock(), record);
        }
    }

    fn flush(&self) {}
}

pub struct File {
    inner: Logger,
    file: Mutex<fs::File>,
}

impl File {
    pub fn new(logger: Logger, file: &Path) -> Result<Self, Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .map_err(|err| Error::Io {
                path: file.into(),
                source: err,
            })?;

        Ok(Self {
            inner: logger,
            file: Mutex::new(file),
        })
    }
}

impl Log for File {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        write_record(self.file.lock().unwrap().deref_mut(), record)
    }

    fn flush(&self) {}
}
