//! The common core of the game systems and components.
//!
//!
#![feature(const_trait_impl)]
#![feature(const_option)]
#![feature(const_mut_refs)]

pub mod actors;
pub mod archive;
pub mod bundles;
pub mod components;
pub mod ecs;
pub mod id;
pub mod localization;
pub mod module;
pub mod net;
pub mod savefile;
pub mod types;
pub mod uuid;
pub mod world;
