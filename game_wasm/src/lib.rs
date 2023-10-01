//! WASM host bindings
#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

#[cfg(test)]
extern crate std;

#[cfg(feature = "raw")]
pub mod raw;
#[cfg(not(feature = "raw"))]
mod raw;

// #[cfg(feature = "panic_handler")]
// mod panic;

pub mod component;
pub mod entity;
pub mod events;
pub mod inventory;
pub mod log;
pub mod math;
pub mod physics;
pub mod process;
pub mod record;
pub mod world;

#[derive(Clone, Debug)]
pub struct Error;
