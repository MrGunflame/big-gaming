use std::collections::HashMap;

use game_common::components::components::RawComponent;
use game_common::components::{PlayerId, Transform};
use game_common::entity::EntityId;
use game_common::world::entity::Entity;
use game_common::world::{CellId, World};
use game_script::WorldProvider;
use game_wasm::components::Component;
use game_wasm::encoding::BinaryWriter;

// TODO: Implement Snapshot-based rollback system.
#[derive(Clone, Debug)]
pub struct WorldState {
    pub world: World,
    pub players: HashMap<PlayerId, EntityId>,
}

impl WorldState {
    pub fn new() -> Self {
        WorldState {
            world: World::new(),
            players: HashMap::new(),
        }
    }

    pub fn spawn(&mut self) -> EntityId {
        self.world.spawn()
    }

    pub fn insert<T: Component>(&mut self, id: EntityId, component: T) {
        let (fields, data) = BinaryWriter::new().encoded(&component);
        self.world
            .insert(id, T::ID, RawComponent::new(data, fields));
    }

    pub fn remove(&mut self, id: EntityId) {
        self.world.despawn(id);
    }

    pub fn get<T: Component>(&self, id: EntityId) -> T {
        let component = self.world.get(id, T::ID).unwrap();
        T::decode(component.reader()).unwrap()
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

    fn player(&self, id: EntityId) -> Option<PlayerId> {
        self.players
            .iter()
            .find(|(player, entity)| **entity == id)
            .map(|(player, _)| *player)
    }
}

pub struct Cell<'a> {
    world: &'a WorldState,
    id: CellId,
}

impl<'a> Cell<'a> {
    pub fn entities(&self) -> CellEntitiesIter<'a> {
        CellEntitiesIter {
            world: self.world,
            iter: self.world.world.iter(),
            cell: self.id,
        }
    }
}

pub struct CellEntitiesIter<'a> {
    world: &'a WorldState,
    iter: game_common::world::Iter<'a>,
    cell: CellId,
}

impl<'a> Iterator for CellEntitiesIter<'a> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.iter.next()?;
            let transform: Transform = self.world.world.get_typed(entity).unwrap();
            if CellId::from(transform.translation) == self.cell {
                return Some(entity);
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
