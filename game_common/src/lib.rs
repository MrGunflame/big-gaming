//! The common core of the game systems and components.
//!
//!
//!

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

// Allow usage of proc macros on the crate.

extern crate self as game_common;

pub mod cell;
pub mod components;
pub mod ecs;
pub mod entity;
pub mod events;
pub mod math;
pub mod metrics;
pub mod module;
pub mod net;
pub mod record;
pub mod units;
pub mod utils;
pub mod world;
