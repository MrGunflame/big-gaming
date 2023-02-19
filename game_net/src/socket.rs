use std::io;
use std::net::{SocketAddr, ToSocketAddrs};

#[cfg(unix)]
use std::os::fd::{AsRawFd, RawFd};

use tokio::net::UdpSocket;

#[derive(Debug)]
pub struct Socket {
    socket: UdpSocket,
}

impl Socket {
    pub fn bind<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let socket = std::net::UdpSocket::bind(addr)?;

        socket.set_nonblocking(true)?;

        Ok(Self {
            socket: UdpSocket::from_std(socket)?,
        })
    }

    pub fn connect<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let socket = std::net::UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0)))?;
        socket.connect(addr)?;
        socket.set_nonblocking(true)?;

        Ok(Self {
            socket: UdpSocket::from_std(socket)?,
        })
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize> {
        self.socket.send_to(buf, target).await
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(buf).await
    }
}

#[cfg(unix)]
impl AsRawFd for Socket {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}
