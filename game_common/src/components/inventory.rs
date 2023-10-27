//! Container inventories

use std::borrow::Borrow;
use std::hash::Hash;
use std::iter::FusedIterator;
use std::num::NonZeroU8;

use ahash::HashMap;
use bytemuck::{Pod, Zeroable};
use thiserror::Error;

use crate::units::Mass;

use super::items::{IntoItemStack, Item, ItemStack};

/// A unique id to a "stack" slot in an inventory.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct InventorySlotId(u64);

impl InventorySlotId {
    #[inline]
    pub const fn into_raw(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn from_raw(bits: u64) -> Self {
        Self(bits)
    }
}

/// A container for storing items. This may be a player inventory or a container in the world.
///
/// Note that the hard limit for a `Inventory` is `usize::MAX` items or total combined mass of
/// `Mass::MAX`, whichever is reached first.
#[derive(Clone, Debug, Default)]
pub struct Inventory {
    items: HashMap<InventorySlotId, ItemStack>,
    /// The count of all items in this `Inventory`.
    count: usize,
    /// The sum of all items in this inventory.
    mass: Mass,
    next_id: u64,
}

impl Inventory {
    /// Creates a new, empty `Inventory`.
    pub fn new() -> Self {
        Self {
            items: HashMap::default(),
            count: 0,
            mass: Mass::new(),
            next_id: 0,
        }
    }

    /// Returns the number of distinct [`ItemStack`]s in this `Inventory`.
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the `Inventory` is emtpy, i.e. contains no [`Item`]s.
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
        T: Borrow<InventorySlotId>,
    {
        self.items.get(id.borrow())
    }

    pub fn get_mut<T>(&mut self, id: T) -> Option<&mut ItemStack>
    where
        T: Borrow<InventorySlotId>,
    {
        self.items.get_mut(id.borrow())
    }

    /// Inserts a new [`Item`] or [`ItemStack`] into the `Inventory`.
    pub fn insert<T>(&mut self, items: T) -> Result<InventorySlotId, InsertionError<T>>
    where
        T: IntoItemStack,
    {
        match self.has_capacity(&items) {
            (false, _) => return Err(InsertionError::MaxItems(items)),
            (_, false) => return Err(InsertionError::MaxMass(items)),
            _ => (),
        }

        Ok(self.insert_stack_unchecked(items.into_item_stack()))
    }

    /// Returns whether we have enough capacity to store the stack.
    fn has_capacity<T>(&self, stack: &T) -> (bool, bool)
    where
        T: IntoItemStack,
    {
        (
            self.count.checked_add(stack.quantity() as usize).is_some(),
            self.mass.checked_add(stack.mass()).is_some(),
        )
    }

    fn insert_stack_unchecked(&mut self, stack: ItemStack) -> InventorySlotId {
        // TODO: Instead of inserting into a new slot we should
        // first check whether we can put the item onto an existing
        // slot.

        let id = InventorySlotId(self.next_id);
        self.next_id += 1;
        assert_ne!(id.0, u64::MAX);

        // Update inventory item quantity.
        self.count += stack.quantity as usize;

        // Update inventory mass.
        self.mass += stack.mass();

        self.items.insert(id, stack);
        id
    }

    /// Removes and returns a single [`Item`] from this `Inventory`. Returns `None` if the item
    /// doesn't exist in this `Inventory`.
    pub fn remove<T>(&mut self, id: T, quantity: u32) -> Option<Item>
    where
        T: Borrow<InventorySlotId>,
    {
        let stack = self.items.get_mut(id.borrow())?;

        // Any stack should always have a quantity greater than zero.
        debug_assert!(stack.quantity >= 1);
        stack.quantity -= quantity;

        self.count -= quantity as usize;
        self.mass -= stack.item.mass;

        if stack.quantity == 0 {
            Some(self.items.remove(id.borrow()).unwrap().item)
        } else {
            Some(stack.item.clone())
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.count = 0;
        self.mass = Mass::new();
        self.next_id = 0;
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.items.iter(),
        }
    }
}

#[derive(Clone, Debug, Error)]
pub enum InsertionError<T> {
    /// The insertion failed because the [`Inventory`] already contains the maximum number of
    /// total items.
    #[error("inventory reached maximum number of items")]
    MaxItems(T),
    /// The insertion failed because the [`Inventory`] already carries the maximum combined
    /// [`Mass`].
    #[error("inventory reached maximum total mass")]
    MaxMass(T),
}

impl<T> InsertionError<T> {
    pub fn into_inner(self) -> T {
        match self {
            Self::MaxItems(inner) => inner,
            Self::MaxMass(inner) => inner,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    inner: std::collections::hash_map::Iter<'a, InventorySlotId, ItemStack>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (InventorySlotId, &'a ItemStack);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (*k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EquipmentSlot(NonZeroU8);

impl EquipmentSlot {
    pub const MAIN_HAND: Self = Self(unsafe { NonZeroU8::new_unchecked(1) });

    pub const HEAD: Self = Self(unsafe { NonZeroU8::new_unchecked(64) });
    pub const EYES: Self = Self(unsafe { NonZeroU8::new_unchecked(65) });
    pub const MASK: Self = Self(unsafe { NonZeroU8::new_unchecked(66) });

    pub const LEFT_ARM: Self = Self(unsafe { NonZeroU8::new_unchecked(67) });
    pub const LEFT_HAND: Self = Self(unsafe { NonZeroU8::new_unchecked(68) });

    pub const RIGHT_ARM: Self = Self(unsafe { NonZeroU8::new_unchecked(69) });
    pub const RIGHT_HAND: Self = Self(unsafe { NonZeroU8::new_unchecked(70) });

    pub const TORSO: Self = Self(unsafe { NonZeroU8::new_unchecked(71) });
    pub const PANTS: Self = Self(unsafe { NonZeroU8::new_unchecked(72) });

    pub const LEFT_LEG: Self = Self(unsafe { NonZeroU8::new_unchecked(73) });
    pub const LEFT_FOOT: Self = Self(unsafe { NonZeroU8::new_unchecked(74) });

    pub const RIGHT_LEG: Self = Self(unsafe { NonZeroU8::new_unchecked(75) });
    pub const RIGHT_FOOT: Self = Self(unsafe { NonZeroU8::new_unchecked(76) });
}

#[cfg(test)]
mod tests {
    use crate::components::components::Components;
    use crate::components::inventory::InsertionError;
    use crate::components::items::{Item, ItemId};
    use crate::record::RecordReference;
    use crate::units::Mass;

    use super::{EquipmentSlot, Inventory};

    #[test]
    fn inventory_insert() {
        let item = new_test_item();
        let mut inventory = Inventory::new();

        inventory.insert(item).unwrap();
    }

    #[test]
    fn inventory_insert_mass_fails() {
        let mut item = new_test_item();
        item.mass = Mass::MAX;

        let mut inventory = Inventory::new();
        inventory.insert(item).unwrap();

        let mut item = new_test_item();
        item.mass = Mass::from_grams(1);

        assert!(matches!(
            inventory.insert(item).unwrap_err(),
            InsertionError::MaxMass(_)
        ));
    }

    // Likely OOM, skip for now
    // #[test]
    // fn inventory_insert_count_fails() {
    //     let mut inventory = Inventory::new();

    //     for _ in 0..u32::MAX {
    //         let item = new_test_item();
    //         inventory.insert(item).unwrap();
    //     }

    //     let item = new_test_item();
    //     assert!(matches!(
    //         inventory.insert(item).unwrap_err(),
    //         InsertionError::MaxItems(_)
    //     ));
    // }

    fn new_test_item() -> Item {
        Item {
            id: ItemId(RecordReference::STUB),
            mass: Mass::new(),
            components: Components::new(),
            equipped: false,
            hidden: false,
        }
    }

    #[test]
    fn test_equipment_slot_consts() {
        // EquipmentSlot constants are guaranteed to be stable.
        macro_rules! assert_equipment_slot_const {
            ($($lhs:expr => $rhs:expr),*,) => {
                $(
                    {
                        assert_eq!($lhs.0.get(), $rhs);
                    }
                )*
            };
        }

        assert_equipment_slot_const! {
            EquipmentSlot::MAIN_HAND => 1,
            EquipmentSlot::HEAD => 64,
            EquipmentSlot::EYES => 65,
            EquipmentSlot::MASK => 66,
            EquipmentSlot::LEFT_ARM => 67,
            EquipmentSlot::LEFT_HAND => 68,
            EquipmentSlot::RIGHT_ARM => 69,
            EquipmentSlot::RIGHT_HAND => 70,
            EquipmentSlot::TORSO => 71,
            EquipmentSlot::PANTS => 72,
            EquipmentSlot::LEFT_LEG => 73,
            EquipmentSlot::LEFT_FOOT => 74,
            EquipmentSlot::RIGHT_LEG => 75,
            EquipmentSlot::RIGHT_FOOT => 76,
        }
    }
}
