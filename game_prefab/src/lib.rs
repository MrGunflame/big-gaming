use std::collections::{HashMap, HashSet, VecDeque};

use game_common::components::components::RawComponent;
use game_common::components::{BinaryReader, Children, Component, Decode, Transform};
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::World;
use game_wasm::encoding::{decode_fields, encode_fields, BinaryWriter};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Default)]
pub struct Prefab {
    // EntityId => [RecordReference => RawComponent]
    entities: HashMap<u64, HashMap<String, EncodedComponent>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EncodedComponent {
    bytes: Vec<u8>,
    fields: Vec<u8>,
}

impl Prefab {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: EntityId, world: &World) {
        let mut components = HashMap::new();
        for (id, component) in world.components(id).iter() {
            components.insert(
                id.to_string(),
                EncodedComponent {
                    bytes: component.as_bytes().to_vec(),
                    fields: encode_fields(component.fields()),
                },
            );
        }

        let index = self.entities.len() as u64;
        self.entities.insert(index, components);
    }

    pub fn instantiate<S>(self, mut world: S) -> EntityId
    where
        S: Spawner,
    {
        let mut entities = HashMap::new();

        let root_entities = self.find_root_entities();
        let mut spawn_queue = VecDeque::new();
        spawn_queue.extend(&root_entities);

        let mut entity_keys = HashMap::new();

        while let Some(entity) = spawn_queue.pop_front() {
            let id = world.spawn();
            entity_keys.insert(entity, id);
            spawn_queue.extend(self.children(entity));
        }

        for (entity, components) in self.entities {
            let entity_id = *entity_keys.get(&entity).unwrap();
            entities.insert(entity, entity_id);

            for (id, component) in components {
                let id: RecordReference = id.parse().unwrap();

                let fields = decode_fields(&component.fields);
                let component = RawComponent::new(component.bytes, fields);

                if id == Children::ID {
                    let reader = BinaryReader::new(
                        component.as_bytes().to_vec(),
                        component.fields().to_vec().into(),
                    );
                    let children = Children::decode(reader).unwrap();

                    let mut new_children = Children::new();
                    for id in children.get() {
                        let id = id.into_raw();
                        let child = *entity_keys.get(&id).unwrap();
                        new_children.insert(child);
                    }

                    let (fields, bytes) = BinaryWriter::new().encoded(&new_children);
                    let component = RawComponent::new(bytes, fields);
                    world.insert(entity_id, id, component);
                    continue;
                }

                world.insert(entity_id, id, component);
            }
        }

        let root = world.spawn();
        let mut children = Children::new();
        for entity in root_entities {
            let id = *entity_keys.get(&entity).unwrap();
            children.insert(id);
        }
        let (fields, bytes) = BinaryWriter::new().encoded(&children);
        let component = RawComponent::new(bytes, fields);
        world.insert(root, Children::ID, component);

        let (fields, bytes) = BinaryWriter::new().encoded(&Transform::default());
        world.insert(root, Transform::ID, RawComponent::new(bytes, fields));

        root
    }

    fn children(&self, parent: u64) -> Vec<u64> {
        let components = self.entities.get(&parent).unwrap();

        let Some(component) = components.get(&Children::ID.to_string()) else {
            return Vec::new();
        };

        let fields = decode_fields(&component.fields);
        let reader = BinaryReader::new(component.bytes.clone(), fields.into());
        let children = Children::decode(reader).unwrap();

        children.get().iter().map(|v| v.into_raw()).collect()
    }

    fn find_root_entities(&self) -> Vec<u64> {
        // Non-root entities are all entities that have a `Children` component
        // pointing at them.
        let mut non_root_entities = HashSet::new();

        for components in self.entities.values() {
            let Some(component) = components.get(&Children::ID.to_string()) else {
                continue;
            };

            let fields = decode_fields(&component.fields);
            let reader = BinaryReader::new(component.bytes.clone(), fields.into());
            let children = Children::decode(reader).unwrap();

            for id in children.get() {
                non_root_entities.insert(id.into_raw());
            }
        }

        let mut root = Vec::with_capacity(self.entities.len() - non_root_entities.len());
        for entity in self.entities.keys() {
            if !non_root_entities.contains(entity) {
                root.push(*entity);
            }
        }

        root
    }

    /// Serializes the `Prefab` into bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(&self.entities).unwrap()
    }

    /// Deserializes the `Prefab` from the given `bytes`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if `bytes` does not contain a valid `Prefab`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let entities = bincode::deserialize(bytes).map_err(Error::Decode)?;
        Ok(Self { entities })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Decode(bincode::Error),
}

pub trait Spawner {
    fn spawn(&mut self) -> EntityId;

    fn insert(&mut self, entity: EntityId, component_id: RecordReference, component: RawComponent);
}

impl<S> Spawner for &mut S
where
    S: Spawner,
{
    fn spawn(&mut self) -> EntityId {
        S::spawn(self)
    }

    fn insert(&mut self, entity: EntityId, component_id: RecordReference, component: RawComponent) {
        S::insert(self, entity, component_id, component)
    }
}

impl Spawner for World {
    fn spawn(&mut self) -> EntityId {
        World::spawn(self)
    }

    fn insert(&mut self, entity: EntityId, component_id: RecordReference, component: RawComponent) {
        World::insert(self, entity, component_id, component)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use game_common::world::World;
    use game_wasm::components::Component;
    use game_wasm::encoding::{encode_fields, BinaryWriter};
    use game_wasm::entity::EntityId;
    use game_wasm::hierarchy::Children;
    use game_wasm::record::{ModuleId, RecordId};
    use game_wasm::world::RecordReference;

    use crate::{EncodedComponent, Prefab};

    #[test]
    fn prefab_instantiate_with_children() {
        const MARKER_COMPONENTS: &[RecordReference] = &[
            RecordReference {
                module: ModuleId::CORE,
                record: RecordId(0x01),
            },
            RecordReference {
                module: ModuleId::CORE,
                record: RecordId(0x02),
            },
            RecordReference {
                module: ModuleId::CORE,
                record: RecordId(0x02),
            },
        ];

        let mut entities = HashMap::new();

        // Top level parent
        {
            let mut components = HashMap::new();
            components.insert(
                MARKER_COMPONENTS[0].to_string(),
                EncodedComponent {
                    bytes: Vec::new(),
                    fields: Vec::new(),
                },
            );

            let mut children = Children::new();
            children.insert(EntityId::from_raw(1));
            let (fields, bytes) = BinaryWriter::new().encoded(&children);
            components.insert(
                Children::ID.to_string(),
                EncodedComponent {
                    bytes,
                    fields: encode_fields(&fields),
                },
            );

            entities.insert(0, components);
        }

        // First Children
        {
            let mut components = HashMap::new();
            components.insert(
                MARKER_COMPONENTS[1].to_string(),
                EncodedComponent {
                    bytes: Vec::new(),
                    fields: Vec::new(),
                },
            );

            let mut children = Children::new();
            children.insert(EntityId::from_raw(2));
            let (fields, bytes) = BinaryWriter::new().encoded(&children);
            components.insert(
                Children::ID.to_string(),
                EncodedComponent {
                    bytes,
                    fields: encode_fields(&fields),
                },
            );

            entities.insert(1, components);
        }

        // Second Children
        {
            let mut components = HashMap::new();
            components.insert(
                MARKER_COMPONENTS[2].to_string(),
                EncodedComponent {
                    bytes: Vec::new(),
                    fields: Vec::new(),
                },
            );

            entities.insert(2, components);
        }

        let prefab = Prefab { entities };

        let mut world = World::new();
        let root = prefab.instantiate(&mut world);

        let root_children = world.get_typed::<Children>(root).unwrap();
        assert_eq!(root_children.len(), 1);

        assert!(world
            .get(root_children.get()[0].into(), MARKER_COMPONENTS[0])
            .is_some());
        let children0 = world
            .get_typed::<Children>(root_children.get()[0].into())
            .unwrap();
        assert_eq!(children0.len(), 1);

        assert!(world
            .get(children0.get()[0].into(), MARKER_COMPONENTS[1])
            .is_some());
        let children1 = world
            .get_typed::<Children>(children0.get()[0].into())
            .unwrap();
        assert_eq!(children1.len(), 1);

        assert!(world
            .get(children1.get()[0].into(), MARKER_COMPONENTS[1])
            .is_some());
        assert!(world
            .get_typed::<Children>(children1.get()[0].into())
            .is_err());
    }
}
