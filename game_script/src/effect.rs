use game_common::entity::EntityId;
use game_common::world::entity::Entity;
use glam::{Quat, Vec3};

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
}

#[derive(Clone, Debug)]
pub enum Effect {
    EntitySpawn(Entity),
    EntityDespawn(EntityId),
    EntityTranslate(EntityId, Vec3),
    EntityRotate(EntityId, Quat),
}
