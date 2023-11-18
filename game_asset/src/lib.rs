//! Asset loader
//!

#![deny(unsafe_op_in_unsafe_fn)]

mod asset;
mod server;

pub use crate::asset::{Asset, Assets, Handle, HandleId};
pub use server::AssetServer;
