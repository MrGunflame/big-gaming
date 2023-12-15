use ahash::HashMap;
use game_common::components::components::Component;
use game_common::components::inventory::{Inventory, InventorySlotId};
use game_common::components::items::ItemStack;
use game_common::components::AsComponent;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::entity::Entity;
use game_common::world::{CellId, World};
use game_script::WorldProvider;

// TODO: Implement Snapshot-based rollback system.
#[derive(Clone, Debug)]
pub struct WorldState {
    next_id: u64,
    inventories: HashMap<EntityId, Inventory>,
    world: World,
}

impl WorldState {
    pub fn new() -> Self {
        WorldState {
            next_id: 0,
            inventories: HashMap::default(),
            world: World::new(),
        }
    }

    pub fn spawn(&mut self) -> EntityId {
        self.world.spawn()
    }

    pub fn insert<T: AsComponent>(&mut self, id: EntityId, component: T) {
        self.world.insert(
            id,
            T::ID,
            Component {
                bytes: component.to_bytes(),
            },
        );
    }

    pub fn remove(&mut self, id: EntityId) {
        self.world.despawn(id);
    }

    pub fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(&id)
    }

    pub fn inventory_mut(&mut self, id: EntityId) -> InventoryMut<'_> {
        debug_assert!(self.world.contains(id));

        let inventory = self.inventories.entry(id).or_default();
        InventoryMut { inventory }
    }

    pub fn insert_inventory(&mut self, id: EntityId, inventory: Inventory) {
        self.inventories.insert(id, inventory);
    }

    pub fn get<T: AsComponent>(&self, id: EntityId) -> T {
        let component = self.world.get(id, T::ID).unwrap();
        T::from_bytes(component.as_bytes())
    }

    pub fn cell(&self, id: CellId) -> Cell<'_> {
        Cell { world: self, id }
    }

    pub fn keys(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.world.iter()
    }
}

impl WorldProvider for WorldState {
    fn world(&self) -> &World {
        &self.world
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
            iter: self.world.world.iter(),
            cell: self.id,
        }
    }
}

pub struct CellEntitiesIter<'a> {
    iter: game_common::world::Iter<'a>,
    cell: CellId,
}

impl<'a> Iterator for CellEntitiesIter<'a> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        // loop {
        //     match self.iter.next() {
        //         Some(id) if CellId::from(entity.transform.translation) == self.cell => {
        //             return Some((*id, entity));
        //         }
        //         None => return None,
        //         _ => (),
        //     }
        // }
        todo!()
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
