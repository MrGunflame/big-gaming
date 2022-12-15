use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use ahash::RandomState;
use bevy_ecs::component::Component;

use crate::localization::LocalizedString;

/// A unique identifier for an item.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u64);

#[derive(Clone, Debug, Component)]
pub struct Inventory {
    items: HashSet<ItemCell, RandomState>,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items: HashSet::with_hasher(RandomState::new()),
        }
    }

    pub fn get<T>(&self, id: T) -> Option<&Item>
    where
        T: Borrow<ItemId>,
    {
        let cell = self.items.get(id.borrow())?;

        // SAFETY: There may not be any mutable references to the item as
        // `self` is borrowed immutably.
        Some(unsafe { cell.get() })
    }

    pub fn get_mut<T>(&mut self, id: T) -> Option<&mut Item>
    where
        T: Borrow<ItemId>,
    {
        let cell = self.items.get(id.borrow())?;

        // SAFETY: There may not be any other references as `self` is borrowed
        // mutably.
        Some(unsafe { cell.get_mut() })
    }
}

#[derive(Debug)]
struct ItemCell(UnsafeCell<Item>);

impl ItemCell {
    fn new(item: Item) -> Self {
        Self(UnsafeCell::new(item))
    }

    /// Returns a reference to the contained item.
    ///
    /// # Safety
    ///
    /// There may not be any mutable reference to the underlying [`Item`] existing.
    #[inline]
    unsafe fn get(&self) -> &Item {
        &*self.0.get()
    }

    /// Returns a mutable reference to the contained [`Item`].
    ///
    /// # Safety
    ///
    /// There may not be any other references to the underlying [`Item`] while the mutable
    /// reference exists.
    #[inline]
    unsafe fn get_mut(&self) -> &mut Item {
        &mut *self.0.get()
    }
}

impl Clone for ItemCell {
    fn clone(&self) -> Self {
        let item = unsafe { self.get() };
        Self::new(item.clone())
    }
}

impl PartialEq for ItemCell {
    fn eq(&self, other: &Self) -> bool {
        let lhs = unsafe { self.get() };
        let rhs = unsafe { other.get() };
        lhs == rhs
    }
}

impl PartialEq<ItemId> for ItemCell {
    fn eq(&self, other: &ItemId) -> bool {
        let lhs = unsafe { self.get() };
        lhs == other
    }
}

impl Hash for ItemCell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let this = unsafe { self.get() };
        this.hash(state);
    }
}

impl Borrow<ItemId> for ItemCell {
    fn borrow(&self) -> &ItemId {
        let this = unsafe { self.get() };
        &this.id
    }
}

impl Eq for ItemCell {}

unsafe impl Send for ItemCell {}
unsafe impl Sync for ItemCell {}

#[derive(Clone, Debug)]
pub struct Item {
    // This field is not public; This is to prevent mutation while the Item
    // exists in a HashSet.
    id: ItemId,
    /// The displayed name of this item.
    pub name: LocalizedString,
    /// The number of items of this type.
    pub quantity: u32,
}

impl Item {
    pub const fn id(&self) -> ItemId {
        self.id
    }
}

impl PartialEq for Item {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialEq<ItemId> for Item {
    #[inline]
    fn eq(&self, other: &ItemId) -> bool {
        self.id == *other
    }
}

impl Eq for Item {}

impl Hash for Item {
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.id.hash(state);
    }
}

impl Borrow<ItemId> for Item {
    #[inline]
    fn borrow(&self) -> &ItemId {
        &self.id
    }
}

/// A list of items currently equipped by a player.
#[derive(Clone, Debug)]
pub struct Equipment(Inventory);
