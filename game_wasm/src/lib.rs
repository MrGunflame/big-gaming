//! WASM host bindings
#![no_std]

#[cfg(feature = "raw")]
pub mod raw;
#[cfg(not(feature = "raw"))]
mod raw;

#[cfg(feature = "panic_handler")]
mod panic;

pub mod log;
pub mod world;

#[derive(Clone, Debug)]
pub struct Error;
