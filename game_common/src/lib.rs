//! The common core of the game systems and components.
//!
//!
//!
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(const_trait_impl)]
#![feature(const_option)]
#![feature(const_mut_refs)]

// Allow usage of proc macros on the crate.

extern crate self as game_common;

pub mod actors;
pub mod archive;
pub mod bundles;
pub mod components;
pub mod ecs;
pub mod entity;
pub mod id;
pub mod localization;
pub mod math;
pub mod metrics;
pub mod module;
pub mod net;
pub mod proto;
pub mod savefile;
pub mod scene;
pub mod units;
pub mod uuid;
pub mod world;
