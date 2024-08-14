//! The world system
//!
//! # World structure
//!
//! The world system is designed to seamlessly handle big open worlds, called [`Level`]s without
//! any loading past the initial loading process (when a player first joins a world).
//!
//! To achieve this, the entire world cannot be loaded at all times. Instead the world is split up
//! into a grid, with each [`Cell`] being loadable and unloadable dynamically when requested.
//!
//! To preserve changes to [`Level`]s, they are serialized into savefiles. This only applies to
//! [`Cell`]s that have been loaded already.
//!
//! # World Generation
//!
//! [`Cell`]s are streamed from a [`Generator`] on demand. This allows any arbitrary [`Level`] to
//! be created. This may include prebuilt worlds, or completely procedually generated [`Level`]s.
//!
//!

pub mod cell;
pub mod control_frame;
pub mod delta_queue;
pub mod entity;
pub mod gen;
pub mod hierarchy;
pub mod interaction;
pub mod snapshot;
pub mod source;
pub mod terrain;
pub mod time;
pub mod world;

use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
pub use cell::{CellId, CELL_SIZE, CELL_SIZE_UINT};
use game_wasm::components::builtin::Transform;
use game_wasm::components::Component;
use game_wasm::encoding::{decode_fields, BinaryReader, BinaryWriter, Decode};
use game_wasm::hierarchy::Children;

use crate::components::components::{Components, RawComponent};
use crate::entity::EntityId;
use crate::record::RecordReference;

pub enum Error<T>
where
    T: Component,
{
    NoComponent,
    Decode(<T as Decode>::Error),
}

impl<T> Clone for Error<T>
where
    T: Component,
    T::Error: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::NoComponent => Self::NoComponent,
            Self::Decode(err) => Self::Decode(err.clone()),
        }
    }
}

impl<T> Debug for Error<T>
where
    T: Component,
    T::Error: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoComponent => {
                write!(f, "NoComponent")
            }
            Self::Decode(err) => {
                write!(f, "Decode({:?})", err)
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct World {
    entities: HashSet<EntityId>,
    next_id: u64,
    components: HashMap<EntityId, Components>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: HashSet::default(),
            components: HashMap::default(),
            next_id: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn spawn(&mut self) -> EntityId {
        let id = EntityId::from_raw(self.next_id);
        self.next_id += 1;

        self.entities.insert(id);
        self.components.insert(id, Components::default());
        id
    }

    pub fn spawn_with_id(&mut self, id: EntityId) {
        self.entities.insert(id);
        self.components.insert(id, Components::default());
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.despawn_recursive(id);
    }

    fn despawn_recursive(&mut self, id: EntityId) {
        let mut despawn_queue = vec![id];

        while let Some(entity) = despawn_queue.pop() {
            if let Ok(children) = self.get_typed::<Children>(entity) {
                for c in children.get() {
                    despawn_queue.push(EntityId::from_raw(c.into_raw()));
                }
            }

            self.entities.remove(&entity);
            self.components.remove(&entity);
        }
    }

    pub fn insert(&mut self, id: EntityId, component_id: RecordReference, component: RawComponent) {
        assert!(self.entities.contains(&id));
        self.components
            .entry(id)
            .or_default()
            .insert(component_id, component);
    }

    pub fn get(&self, id: EntityId, component_id: RecordReference) -> Option<&RawComponent> {
        self.components
            .get(&id)
            .and_then(|components| components.get(component_id))
    }

    pub fn get_mut(
        &mut self,
        id: EntityId,
        component_id: RecordReference,
    ) -> Option<&mut RawComponent> {
        self.components
            .get_mut(&id)
            .and_then(|components| components.get_mut(component_id))
    }

    pub fn remove(&mut self, id: EntityId, component_id: RecordReference) -> Option<RawComponent> {
        self.components
            .get_mut(&id)
            .and_then(|components| components.remove(component_id))
    }

    pub fn insert_typed<T: Component>(&mut self, entity: EntityId, component: T) {
        let (fields, data) = BinaryWriter::new().encoded(&component);
        self.insert(entity, T::ID, RawComponent::new(data, fields));
    }

    pub fn get_typed<T: Component>(&self, entity: EntityId) -> Result<T, Error<T>> {
        let component = self.get(entity, T::ID).ok_or(Error::NoComponent)?;
        let reader = BinaryReader::new(
            component.as_bytes().to_vec(),
            component.fields().to_vec().into(),
        );
        T::decode(reader).map_err(Error::Decode)
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.entities.iter(),
        }
    }

    pub fn components(&self, entity: EntityId) -> &Components {
        self.components.get(&entity).unwrap()
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains(&id)
    }

    pub fn query<Q>(&self) -> Query<'_, Q>
    where
        Q: QueryParams,
    {
        Query {
            iter: self.entities.iter(),
            components: &self.components,
            _marker: PhantomData,
        }
    }

    pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.iter().copied()
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.components.clear();
    }

    pub fn append(&mut self, other: World) -> EntityId {
        let mut entities = HashMap::new();

        let root_entities = other.find_root_entities();
        let mut spawn_queue = VecDeque::new();
        spawn_queue.extend(&root_entities);

        let mut entity_keys = HashMap::new();

        while let Some(entity) = spawn_queue.pop_front() {
            let id = self.spawn();
            entity_keys.insert(entity, id);
            spawn_queue.extend(other.children(entity));
        }

        for (entity, components) in other.components {
            let entity_id = *entity_keys.get(&entity).unwrap();
            entities.insert(entity, entity_id);

            for (id, component) in components.iter() {
                if id == Children::ID {
                    let reader = BinaryReader::new(
                        component.as_bytes().to_vec(),
                        component.fields().to_vec().into(),
                    );
                    let children = Children::decode(reader).unwrap();

                    let mut new_children = Children::new();
                    for id in children.get() {
                        let child = *entity_keys.get(&id).unwrap();
                        new_children.insert(child.into());
                    }

                    let (fields, bytes) = BinaryWriter::new().encoded(&new_children);
                    let component = RawComponent::new(bytes, fields);
                    self.insert(entity_id, id, component);
                    continue;
                }

                self.insert(entity_id, id, component.clone());
            }
        }

        let root = self.spawn();
        let mut children = Children::new();
        for entity in root_entities {
            let id = *entity_keys.get(&entity).unwrap();
            children.insert(id.into());
        }
        let (fields, bytes) = BinaryWriter::new().encoded(&children);
        let component = RawComponent::new(bytes, fields);
        self.insert(root, Children::ID, component);

        root
    }

    fn children(&self, parent: EntityId) -> Vec<EntityId> {
        let components = self.components.get(&parent).unwrap();

        let Some(component) = components.get(Children::ID) else {
            return Vec::new();
        };

        let fields = component.fields();
        let reader = BinaryReader::new(component.as_bytes().to_vec(), fields.to_vec().into());
        let children = Children::decode(reader).unwrap();

        children.get().iter().map(|v| *v).collect()
    }

    fn find_root_entities(&self) -> Vec<EntityId> {
        // Non-root entities are all entities that have a `Children` component
        // pointing at them.
        let mut non_root_entities = HashSet::new();

        for (_, components) in &self.components {
            let Some(component) = components.get(Children::ID) else {
                continue;
            };

            let fields = component.fields();
            let reader = BinaryReader::new(component.as_bytes().to_vec(), fields.to_vec().into());
            let children = Children::decode(reader).unwrap();

            for id in children.get() {
                non_root_entities.insert(*id);
            }
        }

        let mut root = Vec::with_capacity(self.entities.len() - non_root_entities.len());
        for entity in self.entities() {
            if !non_root_entities.contains(&entity) {
                root.push(entity);
            }
        }

        root
    }
}

pub struct Iter<'a> {
    inner: std::collections::hash_set::Iter<'a, EntityId>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().copied()
    }
}

pub trait QueryParams: Sized {
    fn fetch(components: &Components) -> Option<Self>;
}

impl<T> QueryParams for T
where
    T: Component,
{
    fn fetch(components: &Components) -> Option<Self> {
        let component = components.get(T::ID)?;
        let reader = BinaryReader::new(
            component.as_bytes().to_vec(),
            component.fields().to_vec().into(),
        );
        T::decode(reader).ok()
    }
}

pub struct Query<'a, T> {
    iter: std::collections::hash_set::Iter<'a, EntityId>,
    components: &'a HashMap<EntityId, Components>,
    _marker: PhantomData<fn() -> T>,
}

// Transparent Wrapper around `T` to avoid implementing on foreign
// types.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct QueryWrapper<T>(pub T);

impl<T> Deref for QueryWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for QueryWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<C0, C1> QueryParams for QueryWrapper<(C0, C1)>
where
    C0: Component,
    C1: Component,
{
    fn fetch(components: &Components) -> Option<Self> {
        let c0 = components.get(C0::ID)?;
        let c1 = components.get(C1::ID)?;
        let r0 = BinaryReader::new(c0.as_bytes().to_vec(), c0.fields().to_vec().into());
        let r1 = BinaryReader::new(c1.as_bytes().to_vec(), c1.fields().to_vec().into());
        Some(QueryWrapper((C0::decode(r0).ok()?, C1::decode(r1).ok()?)))
    }
}

impl<C0, C1, C2> QueryParams for QueryWrapper<(C0, C1, C2)>
where
    C0: Component,
    C1: Component,
    C2: Component,
{
    fn fetch(components: &Components) -> Option<Self> {
        let c0 = components.get(C0::ID)?;
        let c1 = components.get(C1::ID)?;
        let c2 = components.get(C2::ID)?;
        let r0 = BinaryReader::new(c0.as_bytes().to_vec(), c0.fields().to_vec().into());
        let r1 = BinaryReader::new(c1.as_bytes().to_vec(), c1.fields().to_vec().into());
        let r2 = BinaryReader::new(c2.as_bytes().to_vec(), c2.fields().to_vec().into());
        Some(QueryWrapper((
            C0::decode(r0).ok()?,
            C1::decode(r1).ok()?,
            C2::decode(r2).ok()?,
        )))
    }
}

impl<'a, T> Iterator for Query<'a, T>
where
    T: QueryParams,
{
    type Item = (EntityId, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.iter.next()?;
            let Some(components) = self.components.get(entity) else {
                continue;
            };

            if let Some(query) = T::fetch(components) {
                return Some((*entity, query));
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use game_wasm::components::builtin::Transform;
    use game_wasm::hierarchy::Children;

    use super::World;

    #[test]
    fn world_append() {
        let mut lhs = World::new();
        let mut rhs = World::new();
        let entity = rhs.spawn();
        rhs.insert_typed(entity, Transform::default());

        let id = lhs.append(rhs);
        let children = lhs.get_typed::<Children>(id).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(
            lhs.get_typed::<Transform>(children.get()[0]).unwrap(),
            Transform::default()
        );
    }
}
