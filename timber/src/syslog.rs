use crate::Logger;

use bitflags::bitflags;
use libc::c_int;
use log::{Level, Log, Record};
use std::{
    ffi::{CStr, CString},
    ptr,
};

bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LogOption: i32 {
        const Console = libc::LOG_CONS;
        const NoDelay = libc::LOG_NDELAY;
        const NoWait = libc::LOG_NOWAIT;
        const Delay = libc::LOG_ODELAY;
        const Perror = libc::LOG_PERROR;
        const Pid = libc::LOG_PID;
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(i32)]
pub enum Facility {
    Auth = libc::LOG_AUTH,
    AuthPriv = libc::LOG_AUTHPRIV,
    Cron = libc::LOG_CRON,
    Daemon = libc::LOG_DAEMON,
    Ftp = libc::LOG_FTP,
    Kern = libc::LOG_KERN,
    Local0 = libc::LOG_LOCAL0,
    Local1 = libc::LOG_LOCAL1,
    Local2 = libc::LOG_LOCAL2,
    Local3 = libc::LOG_LOCAL3,
    Local4 = libc::LOG_LOCAL4,
    Local5 = libc::LOG_LOCAL5,
    Local6 = libc::LOG_LOCAL6,
    Local7 = libc::LOG_LOCAL7,
    Lpr = libc::LOG_LPR,
    Mail = libc::LOG_MAIL,
    News = libc::LOG_NEWS,
    Syslog = libc::LOG_SYSLOG,
    #[default]
    User = libc::LOG_USER,
    Uucp = libc::LOG_UUCP,
}

struct Cint(c_int);

impl From<Level> for Cint {
    fn from(value: Level) -> Self {
        Cint(match value {
            Level::Error => libc::LOG_ERR,
            Level::Warn => libc::LOG_WARNING,
            Level::Info => libc::LOG_INFO,
            Level::Debug => libc::LOG_DEBUG,
            Level::Trace => libc::LOG_DEBUG,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub identifier: String,
    pub logopt: LogOption,
    pub facility: Facility,
}

fn openlog(ident: &CStr, logopt: LogOption, facility: Facility) {
    let ident = if ident.is_empty() {
        ptr::null()
    } else {
        ident.as_ptr()
    };
    let logopt = logopt.bits();
    let facility = facility as i32;

    unsafe {
        libc::openlog(ident, logopt, facility);
    }
}

pub(crate) struct Syslog {
    inner: Logger,
    _identifier: CString,
    format: CString,
}

impl Syslog {
    pub fn new(logger: Logger, config: &Config) -> Self {
        let identifier = CString::new(config.identifier.as_str()).unwrap();
        openlog(&identifier, config.logopt, config.facility);

        Self {
            inner: logger,
            _identifier: identifier,
            format: CString::new("%s").unwrap(),
        }
    }
}

impl Log for Syslog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        let message = CString::new(format!("{}", record.args()));
        let priority = Cint::from(match message {
            Ok(_) => record.level(),
            Err(_) => Level::Error,
        })
        .0;
        let message = message.unwrap_or_else(|err| {
            CString::new(format!("Original log message invalid: {err}"))
                .unwrap()
        });

        unsafe {
            libc::syslog(priority, self.format.as_ptr(), message.as_ptr());
        }
    }

    fn flush(&self) {}
}
