use nix::unistd::dup2;
use std::{fs::File, os::fd::AsRawFd, path::Path};

type Error = Box<dyn std::error::Error>;

pub fn null() -> &'static Path {
    Path::new("/dev/null")
}

pub fn root() -> &'static Path {
    Path::new("/")
}

pub fn redirect<T>(old: T, new: &Path) -> Result<(), Error>
where
    T: AsRawFd,
{
    dup2(old.as_raw_fd(), File::open(new)?.as_raw_fd())?;
    Ok(())
}
