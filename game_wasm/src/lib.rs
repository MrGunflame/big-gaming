//! WASM host bindings
#![no_std]

#[cfg(feature = "raw")]
pub mod raw;
#[cfg(not(feature = "raw"))]
mod raw;

pub mod log;
