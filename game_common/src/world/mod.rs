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

pub mod cell;
pub mod chunk;
pub mod component;
pub mod control_frame;
pub mod delta_queue;
pub mod entity;
pub mod gen;
pub mod interaction;
pub mod snapshot;
pub mod source;
pub mod terrain;
pub mod time;
pub mod world;

pub mod inventory;

pub use cell::{CellId, CELL_SIZE, CELL_SIZE_UINT};
