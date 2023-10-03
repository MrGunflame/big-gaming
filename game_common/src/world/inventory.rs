//! Inventory accessors for WorldState.

use std::borrow::Borrow;
use std::ops::Deref;

use crate::components::inventory::{InsertionError, Inventory, InventoryId, ItemMut};
use crate::components::items::Item;
use crate::entity::EntityId;

use super::snapshot::{EntityChange, InventoryItemAdd, InventoryItemRemove};
use super::world::WorldViewMut;

/// A mutable access to an [`Inventory`].
// Note: We don't want to give access to Inventory directly and
// instead go through a wrapper type to prevent directly mutating
// the underlying inventory without telling the world that it changed.
// We also don't want to clone the compare the new inventory on Drop,
// so we don't implement DerefMut.
#[derive(Debug)]
pub struct InventoryMut<'a> {
    entity_id: EntityId,
    inventory: &'a mut Inventory,
    events: &'a mut Vec<EntityChange>,
}

impl<'a> InventoryMut<'a> {
    pub fn insert(&mut self, item: Item) -> Result<InventoryId, InsertionError> {
        let item_id = item.id;

        let id = self.inventory.insert(item)?;

        self.events
            .push(EntityChange::InventoryItemAdd(InventoryItemAdd {
                entity: self.entity_id,
                id,
                item: item_id,
            }));

        Ok(id)
    }

    pub fn get_mut(&mut self, id: InventoryId) -> Option<ItemMut<'_>> {
        self.inventory.get_mut(id)
    }

    #[inline]
    pub fn remove<T>(&mut self, id: T) -> Option<Item>
    where
        T: Borrow<InventoryId>,
    {
        self.remove_in(*id.borrow())
    }

    //     /// Destroys this [`Inventory`], removing it from the entity and destroying all contained
    //     /// items.
    //     pub fn destroy(self) {
    //         self.events
    //             .push(EntityChange::InventoryDestroy(InventoryDestroy {
    //                 entity: self.entity_id,
    //             }));
    //     }

    fn remove_in(&mut self, id: InventoryId) -> Option<Item> {
        let item = self.inventory.remove(id)?;

        self.events
            .push(EntityChange::InventoryItemRemove(InventoryItemRemove {
                entity: self.entity_id,
                id,
            }));

        Some(item)
    }
}

impl<'a> Deref for InventoryMut<'a> {
    type Target = Inventory;

    fn deref(&self) -> &Self::Target {
        &self.inventory
    }
}

#[derive(Debug)]
pub struct InventoriesMut<'r, 'view> {
    pub(crate) view: &'r mut WorldViewMut<'view>,
}

impl<'r, 'view> InventoriesMut<'r, 'view> {
    pub fn get(&self, id: EntityId) -> Option<&Inventory> {
        self.view.snapshot_ref().inventories.get(id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<InventoryMut<'_>> {
        // FIXME: Optimally these two queries should be swapped,
        // failing early if an entity does not have an inventory.
        let events = &mut self.view.new_deltas;

        let inventory = self.view.world.snapshots[self.view.index]
            .inventories
            .get_mut(id)?;

        Some(InventoryMut {
            entity_id: id,
            inventory,
            events,
        })
    }

    pub fn get_mut_or_insert(&mut self, id: EntityId) -> InventoryMut<'_> {
        if self.view.snapshot().inventories.get(id).is_none() {
            self.insert(id, Inventory::new());
        }

        self.get_mut(id).unwrap()
    }

    pub fn insert(&mut self, id: EntityId, inventory: Inventory) {
        assert!(
            inventory.is_empty(),
            "inserted inventories must be empty currently (subject to change)"
        );

        self.view.snapshot().inventories.insert(id, inventory);
    }

    pub fn remove(&mut self, id: EntityId) {
        self.view.snapshot().inventories.remove(id);
    }
}
