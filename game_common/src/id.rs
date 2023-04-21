//! Unique identifiers
//!
//! The engine distinguishes between two different types of ids: [`StrongId`]s are unique
//! identifiers within a module.

mod strong;
mod weak;

pub use strong::StrongId;
pub use weak::WeakId;
