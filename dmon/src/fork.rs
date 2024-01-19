use nix::unistd::{self, setsid, ForkResult};
use std::{
    fs::File,
    io::{self, Read, Write},
    mem::size_of,
    os::fd::{FromRawFd, RawFd},
    process::exit,
};

struct Pipe {
    read: RawFd,
    write: RawFd,
}

impl Pipe {
    fn new() -> Self {
        match unistd::pipe() {
            Ok((read, write)) => Self { read, write },
            Err(err) => {
                eprintln!("Failed to create interprocess channel: {err}");
                exit(1);
            }
        }
    }

    fn read(self) -> RawFd {
        if let Err(err) = unistd::close(self.write) {
            eprintln!("Failed to close write end of pipe: {err}");
            exit(1);
        }

        self.read
    }

    fn write(self) -> RawFd {
        if let Err(err) = unistd::close(self.read) {
            eprintln!("Failed to close read end of pipe: {err}");
            exit(1);
        }

        self.write
    }
}

#[derive(Default)]
pub struct Parent {
    pipe: Option<File>,
}

impl Parent {
    fn from_raw_fd(fd: RawFd) -> Self {
        Self {
            pipe: Some(unsafe { File::from_raw_fd(fd) }),
        }
    }

    pub fn notify(&mut self) -> Result<(), io::Error> {
        self.write("")
    }

    pub fn is_waiting(&self) -> bool {
        self.pipe.is_some()
    }

    pub fn write(&mut self, message: &str) -> Result<(), io::Error> {
        let mut pipe = match self.pipe.take() {
            Some(pipe) => pipe,
            None => return Ok(()),
        };

        let len = message.len().to_ne_bytes();

        pipe.write_all(&len)?;

        if !message.is_empty() {
            write!(pipe, "{message}")?;
        }

        Ok(())
    }
}

struct Child {
    pipe: File,
}

impl Child {
    fn from_raw_fd(fd: RawFd) -> Self {
        Self {
            pipe: unsafe { File::from_raw_fd(fd) },
        }
    }

    fn wait(mut self) -> ! {
        let mut buffer = [0; size_of::<usize>()];
        if let Err(err) = self.pipe.read_exact(&mut buffer) {
            if err.kind() != io::ErrorKind::UnexpectedEof {
                eprintln!("Failed to read data from daemon process: {err}");
            }

            exit(1);
        }

        let expected = match usize::from_ne_bytes(buffer) {
            0 => exit(0),
            len => len,
        };

        let mut message = String::new();
        let len = match self.pipe.read_to_string(&mut message) {
            Ok(len) => len,
            Err(err) => {
                eprintln!("Failed to read message from daemon process: {err}");
                exit(1);
            }
        };

        if len != expected {
            eprintln!(
                "Expected {expected} bytes from daemon process, received {len}"
            );
            exit(1);
        }

        eprintln!("{message}");
        exit(1);
    }
}

fn parent(pipe: Pipe) -> ! {
    Child::from_raw_fd(pipe.read()).wait();
}

fn child(pipe: Pipe) -> Parent {
    let pipe = pipe.write();

    if setsid().is_err() {
        eprintln!("Already process group leader");
        exit(1);
    }

    match unsafe { unistd::fork() } {
        Ok(ForkResult::Parent { .. }) => exit(0),
        Ok(ForkResult::Child) => Parent::from_raw_fd(pipe),
        Err(err) => {
            eprintln!("Failed to fork off for the second time: {err}");
            exit(1);
        }
    }
}

#[must_use]
pub fn fork() -> Parent {
    let pipe = Pipe::new();

    match unsafe { unistd::fork() } {
        Ok(ForkResult::Parent { .. }) => parent(pipe),
        Ok(ForkResult::Child) => child(pipe),
        Err(err) => {
            eprintln!("Failed to fork off for the first time: {err}");
            exit(1);
        }
    }
}
