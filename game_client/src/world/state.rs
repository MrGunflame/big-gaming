use ahash::HashMap;
use game_common::components::inventory::Inventory;
use game_common::entity::EntityId;
use game_common::world::World;

#[derive(Clone, Debug, Default)]
pub struct WorldState {
    pub inventories: Inventories,
    pub world: World,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            inventories: Inventories {
                inventories: HashMap::default(),
            },
            world: World::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Inventories {
    inventories: HashMap<EntityId, Inventory>,
}

impl Inventories {
    pub fn get(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(&id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Inventory> {
        self.inventories.get_mut(&id)
    }

    pub fn insert(&mut self, id: EntityId) {
        self.inventories.insert(id, Inventory::new());
    }

    pub fn remove(&mut self, id: EntityId) {
        self.inventories.remove(&id);
    }
}
