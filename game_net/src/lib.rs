//!
//! The network protocol and implementation.
//!
//! # Networking overview
//!
//! The server/client implementation follows the commonly used system using delta snapshotting,
//! entity interpolation and Lag/Input prediction. See
//! <https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking> for more details.
//!
//! Only entities that carry gameplay meaning are carried over the network. A [`Entity`] contains
//! all data that can be transmitted. Entities that only represent temporary state and graphical
//! effects are not transmitted over the network.
//!
//! # World views
//!
//! The world is stored in [`WorldState`] at a list of snapshots as received/created on the server.
//! The consumer can get a view into a past snapshot using [`WorldState::get`]/
//! [`WorldState::get_mut`].
//!
//! # Lag compensation
//!
pub mod backlog;
mod buffer;
pub mod conn;
pub mod entity;
pub mod host;
pub mod proto;
mod request;
pub mod sequence;
mod serial;
pub mod snapshot;
pub mod socket;
mod stream;
mod validator;

pub use socket::Socket;
