use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};

use crate::record::RecordReference;
use crate::units::Mass;

use super::components::Components;

/// A stack of up to `u32::MAX` items.
///
/// A `ItemStack` can be used as a collection of [`Item`]s when all items are equal (e.g. in
/// inventories), or as a [`Component`] representing items in the world.
#[derive(Clone, Debug)]
pub struct ItemStack {
    pub item: Item,
    // FIXME: Any stack always has a quantity greater than zero,
    // so we can actually make this `NonZero`.
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
#[derive(Clone, Debug)]
pub struct Item {
    pub id: ItemId,
    pub mass: Mass,
    pub components: Components,
    /// Whether the item is currently considered equipped.
    ///
    /// This has no effect if used outside the context of an [`Inventory`].
    pub equipped: bool,
    /// Whether the item should be visible in the player UI.
    ///
    /// This has no effect if used outside the context of an [`Inventory`].
    pub hidden: bool,
}

/// A weak identifer for an item.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct ItemId(pub RecordReference);

/// A component of a modifiable [`Item`].
///
/// Some items (e.g weapons)
pub struct ItemComponent {}

/// A type that can be converted into a [`ItemStack`].
pub trait IntoItemStack {
    fn into_item_stack(self) -> ItemStack;

    fn quantity(&self) -> u32;
    fn mass(&self) -> Mass;
}

impl IntoItemStack for ItemStack {
    fn into_item_stack(self) -> ItemStack {
        self
    }

    fn quantity(&self) -> u32 {
        self.quantity
    }

    fn mass(&self) -> Mass {
        ItemStack::mass(self)
    }
}

impl IntoItemStack for Item {
    fn into_item_stack(self) -> ItemStack {
        ItemStack {
            item: self,
            quantity: 1,
        }
    }

    fn quantity(&self) -> u32 {
        1
    }

    fn mass(&self) -> Mass {
        self.mass
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

#[derive(Copy, Clone, Debug)]
pub struct LoadItem {
    pub id: ItemId,
}

impl LoadItem {
    #[inline]
    pub const fn new(id: ItemId) -> Self {
        Self { id }
    }
}
