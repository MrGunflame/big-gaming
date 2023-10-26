use game_common::components::inventory::InventorySlotId;
use game_common::entity::EntityId;
use game_common::record::RecordReference;

#[derive(Clone, Debug, Default)]
pub struct Dependencies {
    dependencies: Vec<Dependency>,
}

impl Dependencies {
    pub fn push(&mut self, dep: Dependency) {
        self.dependencies.push(dep);
    }

    pub fn dedup(&mut self) {
        self.dependencies.dedup_by(|a, b| match (a, b) {
            (Dependency::Entity(id0), Dependency::Entity(id1)) => id0 == id1,
            (
                Dependency::EntityComponent(entity0, component0),
                Dependency::EntityComponent(entity1, component1),
            ) => entity0 == entity1 && component0 == component1,
            (
                Dependency::InventorySlot(entity0, slot0),
                Dependency::InventorySlot(entity1, slot1),
            ) => entity0 == entity1 && slot0 == slot1,
            (
                Dependency::InventorySlotComponent(entity0, slot0, component0),
                Dependency::InventorySlotComponent(entity1, slot1, component1),
            ) => entity0 == entity1 && slot0 == slot1 && component0 == component1,
            _ => false,
        });
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Dependency {
    Entity(EntityId),
    EntityComponent(EntityId, RecordReference),
    InventorySlot(EntityId, InventorySlotId),
    InventorySlotComponent(EntityId, InventorySlotId, RecordReference),
}
