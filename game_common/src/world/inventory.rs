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
        let cell_id = self.view.get(id)?.cell();
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

pub(crate) fn delta_inventory(
    entity: EntityId,
    prev: &Inventory,
    curr: &Inventory,
) -> Vec<EntityChange> {
    let mut curr = curr.clone();

    let mut events = Vec::new();

    for item in prev.iter() {
        // Item removed between prev and curr.
        if curr.remove(&item.id).is_none() {
            events.push(EntityChange::InventoryItemRemove(InventoryItemRemove {
                entity,
                id: item.id,
            }));
        }

        // TODO: Updated items
    }

    // Items that were missing in prev.
    for item in curr.iter() {
        events.push(EntityChange::InventoryItemAdd(InventoryItemAdd {
            entity,
            id: item.id,
            item: item.item.id,
        }))
    }

    events
}

#[cfg(test)]
mod tests {
    use crate::components::inventory::{Inventory, InventoryId};
    use crate::components::items::{Item, ItemId};
    use crate::entity::EntityId;
    use crate::record::RecordReference;
    use crate::units::Mass;
    use crate::world::snapshot::EntityChange;

    use super::delta_inventory;

    #[test]
    fn delta_inventory_item_added() {
        let entity = EntityId::dangling();
        let item_id = ItemId(RecordReference::STUB);

        let prev = Inventory::new();
        let mut curr = Inventory::new();
        curr.insert(Item {
            id: item_id,
            resistances: None,
            mass: Mass::default(),
            actions: Default::default(),
            components: Default::default(),
            equipped: false,
            hidden: false,
        })
        .unwrap();

        let events = delta_inventory(entity, &prev, &curr);

        assert_eq!(events.len(), 1);
        let event = match &events[0] {
            EntityChange::InventoryItemAdd(event) => event,
            event => panic!(
                "unexpected event {:?}, expected {}",
                event,
                stringify!(EntityChange::InventoryItemAdd),
            ),
        };

        assert_eq!(event.entity, entity);
        assert_eq!(event.id, InventoryId::from_raw(0));
        assert_eq!(event.item, item_id);
    }

    #[test]
    fn delta_inventory_item_removed() {
        let entity = EntityId::dangling();

        let mut prev = Inventory::new();
        let id = prev
            .insert(Item {
                id: ItemId(RecordReference::STUB),
                resistances: None,
                mass: Mass::default(),
                actions: Default::default(),
                components: Default::default(),
                equipped: false,
                hidden: false,
            })
            .unwrap();
        let curr = Inventory::new();

        let events = delta_inventory(entity, &prev, &curr);

        assert_eq!(events.len(), 1);
        let event = match &events[0] {
            EntityChange::InventoryItemRemove(event) => event,
            event => panic!(
                "unexpected event {:?}, expected {}",
                event,
                stringify!(EntityChange::InventoryItemRemove),
            ),
        };

        assert_eq!(event.entity, entity);
        assert_eq!(event.id, id);
    }
}
