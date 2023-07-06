use std::marker::PhantomData;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use game_common::events::EventKind;
use game_common::world::CellId;
use wasmtime::TypedFunc;

/// Events exposed by a script.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Events(u64);

impl Events {
    /// Number of bits used.
    const BITS: u32 = 2;

    pub const NONE: Self = Self(0);

    pub const ACTION: Self = Self(1);
    pub const COLLISION: Self = Self(1 << 1);

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            events: *self,
            index: 0,
            _marker: PhantomData,
        }
    }
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

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    events: Events,
    index: u32,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = EventKind;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < Events::BITS {
            match self.index {
                0 => {
                    if self.events & Events::ACTION != Events::NONE {
                        return Some(EventKind::Action);
                    }
                }
                1 => {
                    if self.events & Events::COLLISION != Events::NONE {
                        return Some(EventKind::Collision);
                    }
                }
                _ => unreachable!(),
            }

            self.index += 1;
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(Events::BITS as usize))
    }
}

///
/// ```ignore
/// fn();
/// ```
pub type OnInit = TypedFunc<(), ()>;

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

///
/// ```ignore
/// fn(item: InventoryId, actor: EntityId);
/// ```
pub type OnEquip = TypedFunc<(u64, u64), ()>;

///
/// ```ignore
/// fn(item: InventoryId, actor: EntityId);
/// ```
pub type OnUnequip = TypedFunc<(u64, u64), ()>;

///
/// ```ignore
/// fn(x: u32, y: u32, z: u32);
/// ```
pub type OnCellLoad = TypedFunc<(u32, u32, u32), ()>;

///
/// ```ignore
/// fn(x: u32, y: u32, z: u32);
/// ```
pub type OnCellUnload = TypedFunc<(u32, u32, u32), ()>;
