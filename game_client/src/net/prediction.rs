use std::collections::HashMap;

use game_common::entity::EntityId;
use game_common::world::world::{AsView, WorldViewMut};
use game_net::snapshot::{Command, CommandId};
use glam::{Quat, Vec3};
use indexmap::IndexMap;

use crate::utils::extract_actor_rotation;

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
            entity.commands.shift_remove(&id);
        }
    }

    pub fn apply(&mut self, view: &mut WorldViewMut<'_>) {
        for (id, pred) in &self.entities {
            let mut entity = view.get_mut(*id).unwrap();

            for cmd in pred.commands.values() {
                match cmd {
                    Command::EntityTranslate(cmd) => {
                        entity.set_translation(cmd.translation);
                    }
                    Command::EntityRotate(cmd) => {
                        entity.set_rotation(cmd.rotation);
                    }
                    _ => todo!(),
                }
            }
        }
    }

    pub fn get_translation<V>(&self, view: V, id: EntityId) -> Option<Vec3>
    where
        V: AsView,
    {
        let predicted_entity = self.entities.get(&id)?;

        let entity = view.get(id)?;

        let mut translation = None;
        for (_, cmd) in predicted_entity.commands.iter() {
            match cmd {
                Command::PlayerMove(cmd) => {
                    // Based on the server impl.
                    let speed = 1.0;
                    let dir = (cmd.bits.forward as u8 as f32) * -Vec3::Z
                        + (cmd.bits.back as u8 as f32) * Vec3::Z
                        + (cmd.bits.left as u8 as f32) * -Vec3::X
                        + (cmd.bits.right as u8 as f32) * Vec3::X;
                    let delta = extract_actor_rotation(entity.transform.rotation) * dir * speed;

                    match &mut translation {
                        Some(translation) => *translation += delta,
                        None => translation = Some(entity.transform.translation + delta),
                    }
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

    pub fn len(&self, id: EntityId) -> Option<usize> {
        Some(self.entities.get(&id).unwrap().commands.len())
    }
}

#[derive(Clone, Debug, Default)]
struct EntityPrediction {
    /// Commands and when they were executed.
    commands: IndexMap<CommandId, Command>,
}
