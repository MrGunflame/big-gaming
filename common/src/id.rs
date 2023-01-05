//! Unique identifiers
//!
//! The engine distinguishes between two different types of ids: [`StrongId`]s are unique
//! identifiers within a module.

mod strong;
mod weak;

pub use strong::StrongId;
pub use weak::WeakId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[deprecated = "Use `StrongId`/`WeakId` instead"]
pub struct NamespacedId<T>(pub T);

impl NamespacedId<u32> {
    /// Creates a new `NamespacedId` using the given parts.
    #[inline]
    pub const fn new(namespace: u16, id: u16) -> Self {
        let namespace = (namespace as u32) << 16;
        let id = id as u32;

        Self(namespace | id)
    }

    pub const fn core(id: u16) -> Self {
        Self::new(0, id)
    }

    /// Returns the namespace component of this `NamespacedId`.
    #[inline]
    pub const fn namespace(self) -> u16 {
        (self.0 >> 16) as u16
    }

    #[inline]
    pub const fn id(self) -> u16 {
        self.0 as u16
    }
}

impl<T> From<T> for NamespacedId<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::NamespacedId;

    #[test]
    fn namespaced_id_u32() {
        let id = NamespacedId::new(15, 31);
        assert_eq!(id.0, 983040 + 31);
        assert_eq!(id.namespace(), 15);
        assert_eq!(id.id(), 31);
    }
}
