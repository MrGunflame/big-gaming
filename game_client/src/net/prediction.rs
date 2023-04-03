use std::collections::HashMap;

use bevy::prelude::{Quat, Vec3};
use game_common::entity::EntityId;
use game_common::world::world::WorldViewMut;
use game_net::snapshot::CommandId;

#[derive(Clone, Debug)]
pub struct LocalOverrides {
    entities: HashMap<EntityId, HashMap<CommandId, Prediction>>,
}

impl LocalOverrides {
    pub fn new() -> Self {
        Self {
            entities: HashMap::default(),
        }
    }

    pub fn push(&mut self, id: EntityId, cmd: CommandId, pred: Prediction) {
        match self.entities.get_mut(&id) {
            Some(entity) => {
                entity.insert(cmd, pred);
            }
            None => {
                self.entities.insert(id, HashMap::from([(cmd, pred)]));
            }
        }
    }

    pub fn remove(&mut self, id: CommandId) {
        let mut empty = vec![];

        for (eid, entity) in self.entities.iter_mut() {
            entity.remove(&id);
            if entity.is_empty() {
                empty.push(*eid);
            }
        }

        for id in empty {
            self.entities.remove(&id);
        }
    }

    pub fn apply(&self, mut view: WorldViewMut<'_>) {
        for (id, entity) in self.entities.iter() {
            let mut ent = view.get_mut(*id).unwrap();

            for pred in entity.values() {
                match pred {
                    Prediction::Translation(translation) => {
                        ent.transform.translation = *translation
                    }
                    Prediction::Rotation(rotation) => ent.transform.rotation = *rotation,
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Prediction {
    Translation(Vec3),
    Rotation(Quat),
}
