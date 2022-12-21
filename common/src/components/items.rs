use bevy_ecs::component::Component;

use crate::id::{NamespacedId, WeakId};
use crate::types::Mass;

use super::combat::Resistances;

/// A stack of up to `u32::MAX` items.
///
/// A `ItemStack` can be used as a collection of [`Item`]s when all items are equal (e.g. in
/// inventories), or as a [`Component`] representing items in the world.
#[derive(Clone, Debug, Component)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: u32,
}

impl ItemStack {
    pub fn mass(&self) -> Mass {
        self.item.mass * self.quantity
    }
}

/// A single item.
///
/// A `Item` can be used in a collection (e.g. in inventories), or as a [`Component`] representing
/// an item in the world.
#[derive(Clone, Debug, Component)]
pub struct Item {
    pub id: ItemId,
    // FIXME: Should better be kv map.
    pub components: Option<Vec<ItemComponentId>>,
    // TODO: Should these really be hardcoded here?
    pub resistances: Option<Resistances>,
    pub ammo: Option<ItemId>,
    pub damage: Option<u32>,
    /// The number of bullets currently in the magazine.
    pub magazine: Option<u32>,
    // pub properties: Properties,
    pub mass: Mass,
}

/// A weak identifer for an item.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub WeakId<u32>);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemComponentId(NamespacedId<u32>);

/// A component of a modifiable [`Item`].
///
/// Some items (e.g weapons)
pub struct ItemComponent {}

/// A type that can be converted into a [`ItemStack`].
pub trait IntoItemStack {
    fn into_item_stack(self) -> ItemStack;
}

impl IntoItemStack for ItemStack {
    fn into_item_stack(self) -> ItemStack {
        self
    }
}

impl IntoItemStack for Item {
    fn into_item_stack(self) -> ItemStack {
        ItemStack {
            item: self,
            quantity: 1,
        }
    }
}

impl From<Item> for ItemStack {
    fn from(value: Item) -> Self {
        Self {
            item: value,
            quantity: 1,
        }
    }
}
