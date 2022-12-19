//! Container inventories

use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::hash::Hash;
use std::num::NonZeroU8;

use ahash::RandomState;
use bevy_ecs::component::Component;

use crate::types::Mass;

use super::items::{IntoItemStack, Item, ItemId, ItemStack};

/// A container for storing items. This may be a player inventory or a container in the world.
#[derive(Clone, Debug, Component)]
pub struct Inventory {
    items: HashMap<ItemId, ItemStack, RandomState>,
    /// The count of all items in this `Inventory`.
    count: usize,
    /// The sum of all items in this inventory.
    mass: Mass,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items: HashMap::with_hasher(RandomState::new()),
            count: 0,
            mass: Mass::new(),
        }
    }

    /// Returns the number of distinct [`ItemStack`]s in this `Inventory`.
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total number of [`Item`]s in this `Inventory`.
    pub fn items(&self) -> usize {
        self.count
    }

    /// Returns the total [`Mass`] sum of all [`Item`]s in this `Inventory`.
    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn get<T>(&self, id: T) -> Option<&ItemStack>
    where
        T: Borrow<ItemId>,
    {
        self.items.get(id.borrow())
    }

    pub fn get_mut<T>(&mut self, id: T) -> Option<&mut ItemStack>
    where
        T: Borrow<ItemId>,
    {
        self.items.get_mut(id.borrow())
    }

    /// Inserts a new [`Item`] or [`ItemStack`] into the `Inventory`.
    pub fn insert<T>(&mut self, items: T)
    where
        T: IntoItemStack,
    {
        let items = items.into_item_stack();

        // Update the mass sum
        self.count += items.quantity as usize;
        self.mass += items.mass();

        match self.get_mut(items.item.id) {
            Some(stack) => {
                stack.quantity += items.quantity;
            }
            None => {
                self.items.insert(items.item.id, items);
            }
        }
    }

    /// Removes and returns a single [`Item`] from this `Inventory`. Returns `None` if the item
    /// doesn't exist in this `Inventory`.
    ///
    /// The returned value is a [`Cow::Borrowed`] if the removed item still remains in the
    /// `Inventory` (only the `quantity` was reduced) and [`Cow::Owned`] if the last item was
    /// removed from the `Inventory`.
    pub fn remove<T>(&mut self, id: T) -> Option<Cow<'_, Item>>
    where
        T: Borrow<ItemId>,
    {
        let stack = self.items.get_mut(id.borrow())?;

        // We always only remove a single item.
        self.count -= 1;
        self.mass -= stack.item.mass;

        if stack.quantity != 1 {
            // Reduce the stack count, then return the item.
            stack.quantity -= 1;

            // Borrow checker trickery: Since we return here early we don't actually
            // keep the borrow of self if this if block never executres.
            // Reborrow stack to satisfy the borrow checker.
            let stack = unsafe { &*(stack as *mut ItemStack) };
            return Some(Cow::Borrowed(&stack.item));
        }

        // Last item from the stack, remove the entry from the map.
        let stack = self.items.remove(id.borrow()).unwrap();
        return Some(Cow::Owned(stack.item));
    }

    /// Removes and returns the whole [`ItemStack`].
    pub fn remove_stack<T>(&mut self, id: T) -> Option<ItemStack>
    where
        T: Borrow<ItemId>,
    {
        let stack = self.items.remove(id.borrow())?;

        // Reduce stack mass.
        self.count -= stack.quantity as usize;
        self.mass -= stack.mass();

        Some(stack)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EquipmentSlot(NonZeroU8);

impl EquipmentSlot {
    pub const HAND: Self = Self(NonZeroU8::new(1).unwrap());

    pub const TORSO: Self = Self(NonZeroU8::new(64).unwrap());
    pub const PANTS: Self = Self(NonZeroU8::new(65).unwrap());
}

/// Inventory of items currently equipped.
#[derive(Clone, Debug, Component)]
pub struct Equipment {
    slots: HashMap<EquipmentSlot, Item>,
}

impl Equipment {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    /// Returns the equipped [`Item`] at the given `slot`. Returns `None` if no [`Item`] is
    /// equipped.
    pub fn get(&self, slot: EquipmentSlot) -> Option<&Item> {
        self.slots.get(&slot)
    }

    /// Removes and returns the equipeed [`Item`] at the given `slot`. Returns `None` if no [`Item`]
    /// is equipped.
    pub fn remove(&mut self, slot: EquipmentSlot) -> Option<Item> {
        self.slots.remove(&slot)
    }

    /// Inserts a new [`Item`] into the given `slot`. Returns the previously equipped [`Item`] at
    /// that slot if present.
    pub fn insert(&mut self, slot: EquipmentSlot, item: Item) -> Option<Item> {
        self.slots.insert(slot, item)
    }
}

impl Default for Equipment {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
