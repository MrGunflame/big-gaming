use std::collections::HashMap;

use game_common::entity::EntityId;
use game_common::world::world::{AsView, WorldViewMut};
use game_net::snapshot::{Command, CommandId};
use glam::{Quat, Vec3};
use indexmap::IndexMap;

#[derive(Clone, Debug, Default)]
pub struct ClientPredictions {
    entities: HashMap<EntityId, EntityPrediction>,
}

impl ClientPredictions {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn push(&mut self, id: EntityId, cmd_id: CommandId, cmd: Command) {
        let entry = self.entities.entry(id).or_default();
        entry.commands.insert(cmd_id, cmd);
    }

    pub fn validate_pre_removal<V>(&self, id: CommandId, view: V)
    where
        V: AsView,
    {
        for (entity_id, entity) in self.entities.iter() {
            for (cmd_id, cmd) in &entity.commands {
                if *cmd_id != id {
                    continue;
                }

                let entity = view.get(*entity_id).unwrap();

                match cmd {
                    Command::EntityTranslate(cmd) => {
                        let server = entity.transform.translation;
                        let client = cmd.translation;

                        tracing::warn!(
                            "client-side misprediction: server is at {:?}, client is at {:?} (diff = {:?})",
                            server,
                            client,
                            server - client,
                        );
                    }
                    Command::EntityRotate(cmd) => {
                        let server = entity.transform.rotation;
                        let client = cmd.rotation;

                        tracing::warn!(
                            "client-side misprediction: server is at {:?}, client is at {:?} (diff = {:?})",
                            server,
                            client,
                            server - client,
                        );
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn remove(&mut self, id: CommandId) {
        for entity in self.entities.values_mut() {
            entity.commands.remove(&id);
        }
    }

    pub fn apply(&mut self, view: &mut WorldViewMut<'_>) {
        for (id, pred) in &self.entities {
            let mut entity = view.get_mut(*id).unwrap();

            for cmd in pred.commands.values() {
                match cmd {
                    Command::EntityTranslate(cmd) => {
                        entity.transform.translation = cmd.translation;
                    }
                    Command::EntityRotate(cmd) => {
                        entity.transform.rotation = cmd.rotation;
                    }
                    _ => todo!(),
                }
            }
        }
    }

    pub fn get_translation(&self, id: EntityId) -> Option<Vec3> {
        let entity = self.entities.get(&id)?;

        let mut translation = None;
        for cmd in entity.commands.values() {
            match cmd {
                Command::EntityTranslate(cmd) => {
                    translation = Some(cmd.translation);
                }
                _ => (),
            }
        }

        translation
    }

    pub fn get_rotation(&self, id: EntityId) -> Option<Quat> {
        let entity = self.entities.get(&id)?;

        let mut rotation = None;
        for cmd in entity.commands.values() {
            match cmd {
                Command::EntityRotate(cmd) => {
                    rotation = Some(cmd.rotation);
                }
                _ => (),
            }
        }

        rotation
    }
}

#[derive(Clone, Debug, Default)]
struct EntityPrediction {
    commands: IndexMap<CommandId, Command>,
}
