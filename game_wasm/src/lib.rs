//! WASM host bindings
#![no_std]

#[deny(unsafe_op_in_unsafe_fn)]
extern crate alloc;

#[cfg(feature = "raw")]
pub mod raw;
#[cfg(not(feature = "raw"))]
mod raw;

// #[cfg(feature = "panic_handler")]
// mod panic;

pub mod events;
pub mod inventory;
pub mod log;
pub mod math;
pub mod process;
pub mod world;

#[derive(Clone, Debug)]
pub struct Error;
