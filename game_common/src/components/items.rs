use std::time::{Duration, Instant};

use bevy_ecs::component::Component;

use crate::id::{NamespacedId, WeakId};
use crate::units::Mass;

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
    pub magazine: Option<Magazine>,
    // pub properties: Properties,
    pub mass: Mass,
    pub cooldown: Cooldown,
}

#[derive(Clone, Debug)]
pub enum Magazine {
    /// A single item.
    Single { id: ItemId, count: u16 },
}

impl Magazine {
    #[inline]
    pub fn pop(&mut self) -> Option<ItemId> {
        match self {
            Self::Single { id, count } => {
                if *count > 0 {
                    *count -= 1;
                    Some(*id)
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    pub fn push(&mut self, item: ItemId, n: u16) {
        match self {
            Self::Single { id, count } => {
                if *id != item {
                    return;
                }

                *count += n;
            }
        }
    }

    #[inline]
    pub fn count(&self) -> u16 {
        match self {
            Self::Single { id: _, count } => *count,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
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

#[derive(Copy, Clone, Debug)]
pub struct Cooldown {
    pub cooldown: Duration,
    pub until: Instant,
}

impl Cooldown {
    #[inline]
    pub fn new(cooldown: Duration) -> Self {
        Self {
            cooldown,
            until: Instant::now(),
        }
    }

    /// Returns `true` whether this cooldown is ready.
    #[inline]
    pub fn is_ready(&mut self) -> bool {
        self.is_ready_in(Instant::now())
    }

    /// Ticks this `Cooldown`, returning whether the cooldown is ready.
    #[inline]
    pub fn tick(&mut self) -> bool {
        // FIXME: This should rather be updated once per ECS tick,
        // rather than for each Cooldown::tick call.

        let now = Instant::now();
        if self.is_ready_in(now) {
            self.until = now + self.cooldown;
            true
        } else {
            false
        }
    }

    #[inline]
    fn is_ready_in(&self, now: Instant) -> bool {
        // Zero cooldown is always ready.
        if self.cooldown.is_zero() {
            true
        } else {
            now > self.until
        }
    }
}

impl Default for Cooldown {
    fn default() -> Self {
        Self {
            cooldown: Duration::ZERO,
            until: Instant::now(),
        }
    }
}

#[derive(Copy, Clone, Debug, Component)]
pub struct LoadItem {
    pub id: ItemId,
}

impl LoadItem {
    #[inline]
    pub const fn new(id: ItemId) -> Self {
        Self { id }
    }
}
