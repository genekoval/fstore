use crate::error::{internal, Error, Result};

use libc::{c_int, LOCK_EX, LOCK_NB, LOCK_UN};
use log::error;
use std::{
    io::{self, ErrorKind},
    os::unix::io::RawFd,
};

pub struct FileLock {
    fd: RawFd,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if let Err(err) = unlock(self.fd) {
            error!("Failed to remove lock for fd ({}): {}", self.fd, err);
        }
    }
}

fn flock(fd: RawFd, flag: c_int) -> io::Result<()> {
    match unsafe { libc::flock(fd, flag) } {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

fn unlock(fd: RawFd) -> io::Result<()> {
    flock(fd, LOCK_UN)
}

pub fn exclusive(fd: RawFd) -> Result<FileLock> {
    if let Err(err) = flock(fd, LOCK_EX | LOCK_NB) {
        match err.kind() {
            ErrorKind::WouldBlock => return Err(Error::WriteLock),
            _ => internal!("Failed to acquire file lock for fd ({fd}): {err}"),
        }
    }

    Ok(FileLock { fd })
}
