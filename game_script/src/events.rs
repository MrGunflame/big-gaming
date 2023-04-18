use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use game_common::entity::EntityId;
use wasmtime::TypedFunc;

/// Events exposed by a script.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Events(u64);

impl Events {
    pub const NONE: Self = Self(0);

    pub const ACTION: Self = Self(1);
    pub const COLLISION: Self = Self(1 << 1);
}

impl BitAnd for Events {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Events {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitOr for Events {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Events {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

///
/// ```ignore
/// fn(entity: EntityId, invoker: EntityId);
/// ```
pub type OnAction = TypedFunc<(u64, u64), ()>;

///
/// ```ignore
/// fn(entity: EntityId, other: EntityId);
/// ```
pub type OnCollision = TypedFunc<(u64, u64), ()>;
