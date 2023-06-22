//! The common core of the game systems and components.
//!
//!
//!
#![deny(unsafe_op_in_unsafe_fn)]

// Allow usage of proc macros on the crate.

extern crate self as game_common;

pub mod archive;
pub mod bundles;
pub mod components;
pub mod ecs;
pub mod entity;
pub mod events;
pub mod hierarchy;
pub mod id;
pub mod localization;
pub mod math;
pub mod metrics;
pub mod module;
pub mod net;
pub mod proto;
pub mod record;
pub mod savefile;
pub mod units;
pub mod world;
