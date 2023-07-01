use std::collections::HashMap;

use game_common::entity::EntityId;
use game_common::world::world::{AsView, WorldViewMut};
use game_net::snapshot::CommandId;
use glam::{Quat, Vec3};

#[derive(Clone, Debug)]
pub struct LocalOverrides {
    entities: HashMap<EntityId, EntityPatches>,
}

impl LocalOverrides {
    pub fn new() -> Self {
        Self {
            entities: HashMap::default(),
        }
    }

    pub fn push(&mut self, id: EntityId, cmd: CommandId, pred: Prediction) {
        let entry = self.entities.entry(id).or_default();

        entry.patches.insert(
            pred.kind(),
            Patch {
                command: cmd,
                value: pred,
            },
        );
    }

    /// Validate an entity before removal.
    pub fn validate_pre_removal<V>(&self, id: CommandId, view: V)
    where
        V: AsView,
    {
        for (entity, entry) in &self.entities {
            for (kind, patch) in &entry.patches {
                if patch.command == id {
                    let entity = view.get(*entity).unwrap();

                    match kind {
                        PredictionKind::Translation => {
                            let server = entity.transform.translation;
                            let client = patch.value.as_translation().unwrap();

                            if server != client {
                                tracing::warn!(
                                    "client-side misprediction: server is at {:?}, client is at {:?} (diff = {:?})",
                                    server,
                                    client,
                                    server - client,
                                );
                            }
                        }
                        PredictionKind::Rotation => {
                            let server = entity.transform.rotation;
                            let client = patch.value.as_rotation().unwrap();

                            if server != client {
                                tracing::warn!(
                                    "client-side misprediction: server is at {:?}, client is at {:?} (diff = {:?})",
                                    server,
                                    client,
                                    server - client,
                                );
                            }
                        }
                    }

                    return;
                }
            }
        }
    }

    pub fn remove(&mut self, id: CommandId) {
        let mut empty = vec![];

        for (eid, entity) in self.entities.iter_mut() {
            entity.patches.retain(|_, patch| patch.command != id);

            if entity.patches.is_empty() {
                empty.push(*eid);
            }
        }

        for id in empty {
            self.entities.remove(&id);
        }
    }

    fn get(&self, entity: EntityId) -> Option<impl Iterator<Item = &Prediction>> {
        self.entities
            .get(&entity)
            .map(|entity| entity.patches.values().map(|patch| &patch.value))
    }

    pub fn apply(&self, mut view: WorldViewMut<'_>) {
        for (id, entity) in self.entities.iter() {
            let mut ent = view.get_mut(*id).unwrap();

            for pred in self.get(*id).unwrap() {
                match pred {
                    Prediction::Translation(translation) => {
                        tracing::trace!(
                            "applying client-side prediction to entity {:?} for translation {:?} -> {:?}",
                            id,
                            ent.transform.translation,
                            translation,
                        );

                        ent.transform.translation = *translation
                    }
                    Prediction::Rotation(rotation) => {
                        tracing::trace!(
                            "applying client-side prediction to entity {:?} for rotation {:?} -> {:?}",
                            id,
                            ent.transform.rotation,
                            rotation,
                        );

                        ent.transform.rotation = *rotation;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
struct EntityPatches {
    patches: HashMap<PredictionKind, Patch>,
}

#[derive(Clone, Debug)]
struct Patch {
    command: CommandId,
    value: Prediction,
}

#[derive(Clone, Debug)]
pub enum Prediction {
    Translation(Vec3),
    Rotation(Quat),
}

impl Prediction {
    fn kind(&self) -> PredictionKind {
        match self {
            Self::Translation(_) => PredictionKind::Translation,
            Self::Rotation(_) => PredictionKind::Rotation,
        }
    }

    fn as_translation(&self) -> Option<Vec3> {
        match self {
            Self::Translation(val) => Some(*val),
            _ => None,
        }
    }

    fn as_rotation(&self) -> Option<Quat> {
        match self {
            Self::Rotation(val) => Some(*val),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum PredictionKind {
    Translation,
    Rotation,
}

#[cfg(test)]
mod tests {
    use game_common::entity::EntityId;
    use game_net::snapshot::CommandId;
    use glam::Vec3;

    use super::{LocalOverrides, Prediction};

    #[test]
    fn local_overrides_overide_newest() {
        let entity = EntityId::from_raw(0);

        let mut overrides = LocalOverrides::new();
        overrides.push(
            entity,
            CommandId(0),
            Prediction::Translation(Vec3::splat(1.0)),
        );

        for patch in overrides.get(entity).unwrap() {
            match patch {
                Prediction::Translation(translation) => assert_eq!(*translation, Vec3::splat(1.0)),
                _ => panic!(
                    "expected Translation(Vec3(1.0, 1.0, 1.0)), but found {:?}",
                    patch
                ),
            }
        }

        // Override previous change
        overrides.push(
            entity,
            CommandId(1),
            Prediction::Translation(Vec3::splat(2.0)),
        );

        for patch in overrides.get(entity).unwrap() {
            match patch {
                Prediction::Translation(translation) => assert_eq!(*translation, Vec3::splat(2.0)),
                _ => panic!(
                    "expected Translation(Vec3(1.0, 1.0, 1.0)), but found {:?}",
                    patch
                ),
            }
        }
    }
}
