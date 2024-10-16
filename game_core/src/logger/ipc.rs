use std::io;
use std::os::fd::{AsFd, OwnedFd};
use std::sync::Mutex;

use rustix::io::{read, write};
use rustix::process::getpid;

static SENDER: Mutex<Option<OwnedFd>> = Mutex::new(None);

#[derive(Debug)]
pub struct Sender {
    fd: OwnedFd,
}

impl Sender {
    pub fn send(&mut self, msg: &[u8]) -> io::Result<()> {
        write_all(&self.fd, &msg.len().to_ne_bytes())?;
        write_all(&self.fd, msg)?;
        Ok(())
    }

    pub fn load() -> Self {
        let mut mutex = SENDER.lock().unwrap();
        let fd = mutex.take().expect("Sender::load can only be called once");
        Self { fd }
    }

    pub fn store(self) {
        dbg!(getpid());
        let mut mutex = SENDER.lock().unwrap();
        *mutex = Some(self.fd);
        dbg!(&mutex);
    }

    pub fn into_fd(self) -> OwnedFd {
        self.fd
    }

    pub fn from_fd(fd: OwnedFd) -> Self {
        Self { fd }
    }
}

#[derive(Debug)]
pub struct Receiver {
    fd: OwnedFd,
}

impl Receiver {
    pub fn recv(&mut self) -> io::Result<Vec<u8>> {
        let mut len = [0; size_of::<usize>()];
        read_exact(&self.fd, &mut len)?;
        let len = usize::from_ne_bytes(len);

        let mut buf = vec![0; len];
        read_exact(&self.fd, &mut buf)?;

        Ok(buf)
    }
}

fn read_exact<F>(fd: F, buf: &mut [u8]) -> io::Result<()>
where
    F: AsFd,
{
    let mut cursor = 0;
    loop {
        let bytes_read = read(&fd, &mut buf[cursor..])?;
        if bytes_read == 0 {
            break;
        }

        cursor += bytes_read;
    }

    Ok(())
}

fn write_all<F>(fd: F, buf: &[u8]) -> io::Result<()>
where
    F: AsFd,
{
    let mut cursor = 0;
    loop {
        let bytes_written = write(&fd, &buf[cursor..])?;
        if bytes_written == 0 {
            break;
        }

        cursor += bytes_written;
    }

    Ok(())
}

pub fn channel() -> io::Result<(Sender, Receiver)> {
    let (tx, rx) = rustix::pipe::pipe()?;
    Ok((Sender { fd: tx }, Receiver { fd: rx }))
}
