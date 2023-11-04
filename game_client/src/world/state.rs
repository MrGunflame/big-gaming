use ahash::HashMap;
use game_common::components::inventory::Inventory;
use game_common::entity::EntityId;
use game_common::world::entity::Entity;

#[derive(Clone, Debug, Default)]
pub struct WorldState {
    pub entities: Entities,
    pub inventories: Inventories,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            entities: Entities {
                entities: HashMap::default(),
                next_id: 0,
            },
            inventories: Inventories {
                inventories: HashMap::default(),
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Entities {
    // FIXME: Replace with generational arena.
    entities: HashMap<EntityId, Entity>,
    next_id: u64,
}

impl Entities {
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
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
