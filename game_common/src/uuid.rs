//! Globally unique identifiers.
//!
//! Unlike types from the [`id`] module, which are optimized for runtime performance and are only
//! unique for a single run, a [`Uuid`] is guaranteed to be unique across multiple runs.
//!
//! [`id`]: crate::id
//!

use snowflaked::sync::Generator;

static GENERATOR: Generator = Generator::new(0);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Uuid(u64);

impl Uuid {
    pub fn new() -> Self {
        Uuid(GENERATOR.generate())
    }
}
