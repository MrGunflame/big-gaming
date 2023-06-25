use std::collections::HashMap;

use game_common::entity::EntityId;
use game_common::world::world::WorldViewMut;
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
                        tracing::info!(
                            "applying client-side prediction to entity {:?} for translation {:?} -> {:?}",
                            id,
                            ent.transform.translation,
                            translation,
                        );

                        ent.transform.translation = *translation
                    }
                    Prediction::Rotation(rotation) => {
                        tracing::info!(
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
