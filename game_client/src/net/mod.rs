mod conn;
mod entities;
mod prediction;
mod snapshot;
mod socket;
pub mod world;

use std::io;

use thiserror::Error;

pub use self::conn::ServerConnection;
pub use self::entities::Entities;

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("failed to bind socket: {0}")]
    Socket(io::Error),
    #[error("empty dns result")]
    EmptyDns,
    #[error("bad socket addr")]
    BadSocketAddr(io::Error),
}

pub use socket::connect_udp;
