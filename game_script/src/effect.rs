use std::sync::Arc;

use game_common::components::components::RawComponent;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_wasm::player::PlayerId;
use game_wasm::resource::RuntimeResourceId;

#[derive(Clone, Debug, Default)]
pub struct Effects {
    effects: Vec<Effect>,
}

impl Effects {
    pub fn push(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn into_iter(self) -> impl Iterator<Item = Effect> {
        self.effects.into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &'_ Effect> + '_ {
        self.effects.iter()
    }
}

#[derive(Clone, Debug)]
pub enum Effect {
    EntitySpawn(EntityId),
    EntityDespawn(EntityId),
    EntityComponentInsert(EntityComponentInsert),
    EntityComponentRemove(EntityComponentRemove),
    PlayerSetActive(PlayerSetActive),
    CreateResource(CreateResource),
    DestroyResource(DestroyResource),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerSetActive {
    pub player: PlayerId,
    pub entity: EntityId,
}

#[derive(Clone, Debug)]
pub struct EntityComponentInsert {
    pub entity: EntityId,
    pub component_id: RecordReference,
    pub component: RawComponent,
}

#[derive(Clone, Debug)]
pub struct EntityComponentRemove {
    pub entity: EntityId,
    pub component_id: RecordReference,
}

#[derive(Clone, Debug)]
pub struct CreateResource {
    pub id: RuntimeResourceId,
    pub data: Arc<[u8]>,
}

#[derive(Clone, Debug)]
pub struct DestroyResource {
    pub id: RuntimeResourceId,
}
