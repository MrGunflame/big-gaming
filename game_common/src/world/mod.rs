//! The world system
//!
//! # World structure
//!
//! The world system is designed to seamlessly handle big open worlds, called [`Level`]s without
//! any loading past the initial loading process (when a player first joins a world).
//!
//! To achieve this, the entire world cannot be loaded at all times. Instead the world is split up
//! into a grid, with each [`Cell`] being loadable and unloadable dynamically when requested.
//!
//! To preserve changes to [`Level`]s, they are serialized into savefiles. This only applies to
//! [`Cell`]s that have been loaded already.
//!
//! # World Generation
//!
//! [`Cell`]s are streamed from a [`Generator`] on demand. This allows any arbitrary [`Level`] to
//! be created. This may include prebuilt worlds, or completely procedually generated [`Level`]s.
//!
//!

mod cell;
pub mod chunk;
pub mod component;
pub mod gen;
pub mod interaction;
mod level;
pub mod source;
pub mod time;

pub use cell::{Cell, CellId, CELL_SIZE};
pub use level::Level;
