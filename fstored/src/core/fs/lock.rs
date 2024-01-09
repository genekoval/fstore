use libc::{c_int, LOCK_EX, LOCK_NB, LOCK_UN};
use log::error;
use std::{
    io::{Error, Result},
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

fn flock(fd: RawFd, flag: c_int) -> Result<()> {
    match unsafe { libc::flock(fd, flag) } {
        0 => Ok(()),
        _ => Err(Error::last_os_error()),
    }
}

fn unlock(fd: RawFd) -> Result<()> {
    flock(fd, LOCK_UN)
}

pub fn exclusive(fd: RawFd) -> Result<FileLock> {
    flock(fd, LOCK_EX | LOCK_NB)?;
    Ok(FileLock { fd })
}
