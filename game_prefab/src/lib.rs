mod format;

use std::collections::HashMap;
use std::ops::Range;

use game_common::components::components::RawComponent;
use game_common::components::{Children, Component};
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::World;
use game_tracing::trace_span;
use game_wasm::encoding::{decode_fields, encode_fields, BinaryWriter};

pub use format::DecodeError;

#[derive(Clone, Debug, Default)]
pub struct Prefab {
    entities: Vec<Vec<ComponentRef>>,
    children: HashMap<u64, Vec<u64>>,
    root: Vec<u64>,
    data: Vec<u8>,
}

impl Prefab {
    /// Creates a new, empty `Prefab`.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            children: HashMap::new(),
            data: Vec::new(),
            root: Vec::new(),
        }
    }

    pub fn add(&mut self, id: EntityId, world: &World) {
        let _span = trace_span!("Prefab::add").entered();

        let mut entities = Vec::new();

        // Collect all recursive children of `id` in the stack
        // `entities` "bottom-up". This means popping from
        // `entities` will always yield entities whose children
        // have already been yielded.
        let mut stack = vec![id];
        while let Some(id) = stack.pop() {
            entities.push(id);

            if let Ok(children) = world.get_typed::<Children>(id) {
                stack.extend(children.get());
            }
        }

        let mut spawned_entities = HashMap::new();

        while let Some(entity) = entities.pop() {
            let index = self.entities.len();
            spawned_entities.insert(entity, index as u64);

            let mut components = Vec::new();
            for (component_id, component) in world.components(entity).iter() {
                // We handle the `Children` component manually.
                if component_id == Children::ID {
                    continue;
                }

                let data = component.as_bytes().to_vec();
                let fields = encode_fields(component.fields());

                let data_start = self.data.len();
                self.data.extend(data);
                let data_end = self.data.len();

                let fields_start = self.data.len();
                self.data.extend(fields);
                let fields_end = self.data.len();

                components.push(ComponentRef {
                    id: component_id,
                    data: Range {
                        start: data_start,
                        end: data_end,
                    },
                    fields: Range {
                        start: fields_start,
                        end: fields_end,
                    },
                });
            }

            self.entities.push(components);

            if let Ok(children) = world.get_typed::<Children>(entity) {
                // `entities` is order so that all entities that are children
                // of the current entity have already been processed.
                let children_list = children
                    .get()
                    .iter()
                    .map(|id| *spawned_entities.get(id).unwrap())
                    .collect();

                self.children.insert(index as u64, children_list);
            }
        }

        let root = spawned_entities.get(&id).unwrap();
        self.root.push(*root);
    }

    /// Instantiate the `Prefab` using the given [`Spawner`] and returns the [`EntityId`] of the
    /// spawned prefab.
    pub fn instantiate<S>(self, mut spawner: S) -> EntityId
    where
        S: Spawner,
    {
        let _span = trace_span!("Prefab::instantiate").entered();

        let mut entities = Vec::new();

        let mut stack = self.root.clone();
        while let Some(index) = stack.pop() {
            entities.push(index);

            if let Some(children) = self.children.get(&index) {
                stack.extend(children);
            }
        }

        let mut spawned_entities = HashMap::new();

        while let Some(index) = entities.pop() {
            let entity = spawner.spawn();
            spawned_entities.insert(index, entity);

            let component_refs = &self.entities[index as usize];
            for component_ref in component_refs {
                let component = component_ref.load(&self.data);
                spawner.insert(entity, component_ref.id, component);
            }

            if let Some(children) = self.children.get(&index) {
                let mut children_component = Children::new();

                for children in children {
                    let children_entity = spawned_entities.get(children).unwrap();
                    children_component.insert(*children_entity);
                }

                if !children_component.is_empty() {
                    spawner.insert_typed(entity, children_component);
                }
            }
        }

        let root_entity = spawner.spawn();

        let mut children_component = Children::new();
        for index in &self.root {
            let children_entity = spawned_entities.get(index).unwrap();
            children_component.insert(*children_entity);
        }

        if !children_component.is_empty() {
            spawner.insert_typed(root_entity, children_component);
        }

        root_entity
    }

    /// Serializes the `Prefab` into bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        format::encode(self)
    }

    /// Deserializes the `Prefab` from the given `bytes`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if `bytes` does not contain a valid `Prefab`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        format::decode(bytes)
    }
}

#[derive(Clone, Debug)]
struct ComponentRef {
    id: RecordReference,
    data: Range<usize>,
    fields: Range<usize>,
}

impl ComponentRef {
    fn load(&self, buf: &[u8]) -> RawComponent {
        let data = &buf[self.data.clone()];
        let fields = &buf[self.fields.clone()];
        let fields = decode_fields(fields);
        RawComponent::new(data, fields)
    }
}

pub trait Spawner {
    fn spawn(&mut self) -> EntityId;

    fn insert(&mut self, entity: EntityId, component_id: RecordReference, component: RawComponent);

    fn insert_typed<T: Component>(&mut self, entity: EntityId, component: T) {
        let (fields, data) = BinaryWriter::new().encoded(&component);
        self.insert(entity, T::ID, RawComponent::new(data, fields));
    }
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
    use game_common::world::World;
    use game_wasm::hierarchy::Children;
    use game_wasm::record::{ModuleId, RecordId};
    use game_wasm::world::RecordReference;

    use crate::{ComponentRef, Prefab};

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

        let prefab = Prefab {
            entities: vec![
                // Top level parent
                vec![ComponentRef {
                    id: MARKER_COMPONENTS[0],
                    data: 0..0,
                    fields: 0..0,
                }],
                // First children
                vec![ComponentRef {
                    id: MARKER_COMPONENTS[1],
                    data: 0..0,
                    fields: 0..0,
                }],
                // Second children
                vec![ComponentRef {
                    id: MARKER_COMPONENTS[2],
                    data: 0..0,
                    fields: 0..0,
                }],
            ],
            children: [(0, vec![1]), (1, vec![2])].into(),
            root: vec![0],
            data: Vec::new(),
        };

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
