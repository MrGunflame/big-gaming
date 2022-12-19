use bevy_ecs::component::Component;

use crate::id::NamespacedId;
use crate::localization::LocalizedString;

use super::combat::Resistances;

/// A stack of up to `u32::MAX` items.
///
/// A `ItemStack` can be used as a collection of [`Item`]s when all items are equal (e.g. in
/// inventories), or as a [`Component`] representing items in the world.
#[derive(Clone, Debug, Component)]
pub struct ItemStack {
    pub id: Item,
    pub quantity: u32,
}

/// A single item.
///
/// A `Item` can be used in a collection (e.g. in inventories), or as a [`Component`] representing
/// an item in the world.
#[derive(Clone, Debug, Component)]
pub struct Item {
    pub id: ItemId,
    pub name: LocalizedString,
    // FIXME: Should better be kv map.
    pub components: Option<Vec<ItemComponentId>>,
    // TODO: Should these really be hardcoded here?
    pub resistances: Option<Resistances>,
    pub ammo: Option<ItemId>,
    pub damage: Option<u32>,
    pub magazine: Option<u32>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(NamespacedId<u32>);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemComponentId(NamespacedId<u32>);

/// A component of a modifiable [`Item`].
///
/// Some items (e.g weapons)
pub struct ItemComponent {}

impl ItemId {}
