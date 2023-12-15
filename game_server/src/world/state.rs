use ahash::HashMap;
use game_common::components::components::Component;
use game_common::components::inventory::{Inventory, InventorySlotId};
use game_common::components::items::ItemStack;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::entity::Entity;
use game_common::world::CellId;
use game_script::WorldProvider;

// TODO: Implement Snapshot-based rollback system.
#[derive(Clone, Debug)]
pub struct WorldState {
    next_id: u64,
    entities: HashMap<EntityId, Entity>,
    inventories: HashMap<EntityId, Inventory>,
}

impl WorldState {
    pub fn new() -> Self {
        WorldState {
            next_id: 0,
            entities: HashMap::default(),
            inventories: HashMap::default(),
        }
    }

    pub fn insert(&mut self, mut entity: Entity) -> EntityId {
        let id = EntityId::from_raw(self.next_id);
        self.next_id += 1;

        entity.id = id;
        self.entities.insert(id, entity);
        id
    }

    pub fn remove(&mut self, id: EntityId) -> Option<Entity> {
        self.entities.remove(&id)
    }

    pub fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(&id)
    }

    pub fn inventory_mut(&mut self, id: EntityId) -> InventoryMut<'_> {
        debug_assert!(self.entities.contains_key(&id));

        let inventory = self.inventories.entry(id).or_default();
        InventoryMut { inventory }
    }

    pub fn insert_inventory(&mut self, id: EntityId, inventory: Inventory) {
        self.inventories.insert(id, inventory);
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn cell(&self, id: CellId) -> Cell<'_> {
        Cell { world: self, id }
    }

    pub fn keys(&self) -> Keys<'_> {
        Keys {
            iter: self.entities.keys(),
        }
    }
}

impl WorldProvider for WorldState {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(&id)
    }
}

pub struct InventoryMut<'a> {
    inventory: &'a mut Inventory,
}

impl<'a> InventoryMut<'a> {
    pub fn insert(&mut self, stack: ItemStack) -> InventorySlotId {
        self.inventory.insert(stack).unwrap()
    }

    pub fn remove(&mut self, id: InventorySlotId) {
        self.inventory.remove(id, u32::MAX);
    }

    pub fn get_mut(&mut self, id: InventorySlotId) -> ItemStackMut<'_> {
        let stack = self.inventory.get_mut(id).unwrap();
        ItemStackMut { stack }
    }

    pub fn clear(&mut self) {
        self.inventory.clear();
    }
}

pub struct ItemStackMut<'a> {
    stack: &'a mut ItemStack,
}

impl<'a> ItemStackMut<'a> {
    pub fn set_equipped(&mut self, equipped: bool) {
        self.stack.item.equipped = equipped;
    }

    pub fn component_insert(&mut self, id: RecordReference, component: Component) {
        self.stack.item.components.insert(id, component);
    }

    pub fn component_remove(&mut self, id: RecordReference) {
        self.stack.item.components.remove(id);
    }
}

pub struct Cell<'a> {
    world: &'a WorldState,
    id: CellId,
}

impl<'a> Cell<'a> {
    pub fn entities(&self) -> CellEntitiesIter<'a> {
        CellEntitiesIter {
            iter: self.world.entities.iter(),
            cell: self.id,
        }
    }
}

pub struct CellEntitiesIter<'a> {
    iter: std::collections::hash_map::Iter<'a, EntityId, Entity>,
    cell: CellId,
}

impl<'a> Iterator for CellEntitiesIter<'a> {
    type Item = (EntityId, &'a Entity);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((id, entity)) if CellId::from(entity.transform.translation) == self.cell => {
                    return Some((*id, entity));
                }
                None => return None,
                _ => (),
            }
        }
    }
}

pub struct Keys<'a> {
    iter: std::collections::hash_map::Keys<'a, EntityId, Entity>,
}

impl<'a> Iterator for Keys<'a> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
