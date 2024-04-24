//! The common core of the game systems and components.
//!
//!
//!

// Allow usage of proc macros on the crate.

extern crate self as game_common;

pub mod cell;
pub mod collections;
pub mod components;
pub mod entity;
pub mod events;
pub mod hex;
pub mod math;
pub mod metrics;
pub mod module;
pub mod net;
pub mod record;
pub mod sync;
pub mod units;
pub mod utils;
pub mod world;
