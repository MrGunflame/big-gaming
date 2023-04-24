//! Container inventories

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::FusedIterator;
use std::num::NonZeroU8;

use ahash::RandomState;
use bevy_ecs::component::Component;
use bytemuck::{Pod, Zeroable};
use indexmap::IndexMap;

use crate::units::Mass;

use super::items::Item;

/// A unique id refering an item inside exactly one inventory.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct InventoryId(u64);

impl InventoryId {
    #[inline]
    pub const fn into_bits(self) -> u64 {
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
#[derive(Clone, Debug, Default, Component)]
pub struct Inventory {
    items: IndexMap<InventoryId, Item, RandomState>,
    /// The count of all items in this `Inventory`.
    count: usize,
    /// The sum of all items in this inventory.
    mass: Mass,
    next_id: InventoryId,
}

impl Inventory {
    /// Creates a new, empty `Inventory`.
    pub fn new() -> Self {
        Self {
            items: IndexMap::with_hasher(RandomState::new()),
            count: 0,
            mass: Mass::new(),
            next_id: InventoryId(0),
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

    pub fn get<T>(&self, id: T) -> Option<&Item>
    where
        T: Borrow<InventoryId>,
    {
        self.items.get(id.borrow())
    }

    pub fn get_mut<T>(&mut self, id: T) -> Option<&mut Item>
    where
        T: Borrow<InventoryId>,
    {
        self.items.get_mut(id.borrow())
    }

    /// Inserts a new [`Item`] or [`ItemStack`] into the `Inventory`.
    pub fn insert(&mut self, item: Item) -> Result<InventoryId, InsertionError> {
        let id = self.next_id;
        self.next_id.0 += 1;

        // Update inventory item quantity.
        match self.count.checked_add(1) {
            Some(count) => self.count = count,
            None => return Err(InsertionError::MaxItems(item)),
        }

        // Update inventory mass.
        match self.mass.checked_add(item.mass) {
            Some(mass) => self.mass = mass,
            None => return Err(InsertionError::MaxMass(item)),
        }

        self.items.insert(id, item);

        Ok(id)
    }

    /// Removes and returns a single [`Item`] from this `Inventory`. Returns `None` if the item
    /// doesn't exist in this `Inventory`.
    ///
    /// The returned value is a [`Cow::Borrowed`] if the removed item still remains in the
    /// `Inventory` (only the `quantity` was reduced) and [`Cow::Owned`] if the last item was
    /// removed from the `Inventory`.
    pub fn remove<T>(&mut self, id: T) -> Option<Item>
    where
        T: Borrow<InventoryId>,
    {
        let item = self.items.get_mut(id.borrow())?;

        // We always only remove a single item.
        self.count -= 1;
        self.mass -= item.mass;

        Some(self.items.remove(id.borrow()).unwrap())
    }

    pub fn sort_by<F>(&mut self, mut f: F)
    where
        F: FnMut(&Item, &Item) -> Ordering,
    {
        self.items.sort_by(|_, lhs, _, rhs| f(lhs, rhs))
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.items.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a Inventory {
    type Item = ItemRef<'a>;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Debug)]
pub enum InsertionError {
    /// The insertion failed because the [`Inventory`] already contains the maximum number of
    /// total items.
    MaxItems(Item),
    /// The insertion failed because the [`Inventory`] already carries the maximum combined
    /// [`Mass`].
    MaxMass(Item),
}

impl InsertionError {
    pub fn into_inner(self) -> Item {
        match self {
            Self::MaxItems(inner) => inner,
            Self::MaxMass(inner) => inner,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EquipmentSlot(NonZeroU8);

impl EquipmentSlot {
    pub const MAIN_HAND: Self = Self(NonZeroU8::new(1).unwrap());

    pub const HEAD: Self = Self(NonZeroU8::new(64).unwrap());
    pub const EYES: Self = Self(NonZeroU8::new(65).unwrap());
    pub const MASK: Self = Self(NonZeroU8::new(66).unwrap());

    pub const LEFT_ARM: Self = Self(NonZeroU8::new(67).unwrap());
    pub const LEFT_HAND: Self = Self(NonZeroU8::new(68).unwrap());

    pub const RIGHT_ARM: Self = Self(NonZeroU8::new(69).unwrap());
    pub const RIGHT_HAND: Self = Self(NonZeroU8::new(70).unwrap());

    pub const TORSO: Self = Self(NonZeroU8::new(71).unwrap());
    pub const PANTS: Self = Self(NonZeroU8::new(72).unwrap());

    pub const LEFT_LEG: Self = Self(NonZeroU8::new(73).unwrap());
    pub const LEFT_FOOT: Self = Self(NonZeroU8::new(74).unwrap());

    pub const RIGHT_LEG: Self = Self(NonZeroU8::new(75).unwrap());
    pub const RIGHT_FOOT: Self = Self(NonZeroU8::new(76).unwrap());
}

/// An inventory equipped [`Item`]s.
///
/// `Equipment` is very similar to [`Inventory`] with a few key differences:
/// - `Equipment` is indexed by a [`EquipmentSlot`].
/// - `Equipment` can only contain a single [`Item`] in a cell (unlike [`Inventory`], which can
/// contain an [`ItemStack`]).
#[derive(Clone, Debug, Component)]
pub struct Equipment {
    slots: HashMap<EquipmentSlot, Item, RandomState>,
    mass: Mass,
}

impl Equipment {
    pub fn new() -> Self {
        Self {
            slots: HashMap::with_hasher(RandomState::new()),
            mass: Mass::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    /// Returns the equipped [`Item`] at the given `slot`. Returns `None` if no [`Item`] is
    /// equipped.
    pub fn get(&self, slot: EquipmentSlot) -> Option<&Item> {
        self.slots.get(&slot)
    }

    pub fn get_mut(&mut self, slot: EquipmentSlot) -> Option<&mut Item> {
        self.slots.get_mut(&slot)
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

    #[inline]
    pub fn iter(&self) -> EquipmentIter<'_> {
        EquipmentIter {
            iter: self.slots.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a Equipment {
    type Item = &'a Item;
    type IntoIter = EquipmentIter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Default for Equipment {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct EquipmentIter<'a> {
    iter: std::collections::hash_map::Iter<'a, EquipmentSlot, Item>,
}

impl<'a> Iterator for EquipmentIter<'a> {
    type Item = &'a Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for EquipmentIter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a> FusedIterator for EquipmentIter<'a> {}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    iter: indexmap::map::Iter<'a, InventoryId, Item>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = ItemRef<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(id, item)| ItemRef { id: *id, item })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}

#[derive(Copy, Clone, Debug)]
pub struct ItemRef<'a> {
    pub id: InventoryId,
    pub item: &'a Item,
}

#[cfg(test)]
mod tests {
    use crate::components::actions::Actions;
    use crate::components::components::{Components, RecordReference};
    use crate::components::inventory::InsertionError;
    use crate::components::items::{Item, ItemId};
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
            resistances: None,
            actions: Actions::new(),
            components: Components::new(),
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
