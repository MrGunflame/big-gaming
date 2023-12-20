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
pub mod interaction;
pub mod snapshot;
pub mod source;
pub mod terrain;
pub mod time;
pub mod world;

use std::marker::PhantomData;

use ahash::{HashMap, HashSet};
pub use cell::{CellId, CELL_SIZE, CELL_SIZE_UINT};

use crate::components::components::{Component, Components};
use crate::components::AsComponent;
use crate::entity::EntityId;
use crate::record::RecordReference;

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
        self.entities.remove(&id);
        self.components.remove(&id);
    }

    pub fn insert(&mut self, id: EntityId, component_id: RecordReference, component: Component) {
        assert!(self.entities.contains(&id));
        self.components
            .entry(id)
            .or_default()
            .insert(component_id, component);
    }

    pub fn get(&self, id: EntityId, component_id: RecordReference) -> Option<&Component> {
        self.components
            .get(&id)
            .and_then(|components| components.get(component_id))
    }

    pub fn get_mut(
        &mut self,
        id: EntityId,
        component_id: RecordReference,
    ) -> Option<&mut Component> {
        self.components
            .get_mut(&id)
            .and_then(|components| components.get_mut(component_id))
    }

    pub fn remove(&mut self, id: EntityId, component_id: RecordReference) -> Option<Component> {
        self.components
            .get_mut(&id)
            .and_then(|components| components.remove(component_id))
    }

    pub fn insert_typed<T: AsComponent>(&mut self, entity: EntityId, component: T) {
        let component_id = T::ID;
        let component = Component::new(component.to_bytes());

        self.insert(entity, component_id, component);
    }

    pub fn get_typed<T: AsComponent>(&self, entity: EntityId) -> T {
        T::from_bytes(self.get(entity, T::ID).unwrap().as_bytes())
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

    pub fn clear(&mut self) {
        self.entities.clear();
        self.components.clear();
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
    T: AsComponent,
{
    fn fetch(components: &Components) -> Option<Self> {
        let component = components.get(T::ID)?;
        Some(T::from_bytes(component.as_bytes()))
    }
}

pub struct Query<'a, T> {
    iter: std::collections::hash_set::Iter<'a, EntityId>,
    components: &'a HashMap<EntityId, Components>,
    _marker: PhantomData<fn() -> T>,
}

impl<C0, C1> QueryParams for (C0, C1)
where
    C0: AsComponent,
    C1: AsComponent,
{
    fn fetch(components: &Components) -> Option<Self> {
        let c0 = components.get(C0::ID)?;
        let c1 = components.get(C1::ID)?;
        Some((C0::from_bytes(c0.as_bytes()), C1::from_bytes(c1.as_bytes())))
    }
}

impl<C0, C1, C2> QueryParams for (C0, C1, C2)
where
    C0: AsComponent,
    C1: AsComponent,
    C2: AsComponent,
{
    fn fetch(components: &Components) -> Option<Self> {
        let c0 = components.get(C0::ID)?;
        let c1 = components.get(C1::ID)?;
        let c2 = components.get(C2::ID)?;
        Some((
            C0::from_bytes(c0.as_bytes()),
            C1::from_bytes(c1.as_bytes()),
            C2::from_bytes(c2.as_bytes()),
        ))
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
