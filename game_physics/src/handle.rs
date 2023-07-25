use std::collections::HashMap;
use std::hash::Hash;

use game_common::entity::EntityId;

#[derive(Clone, Debug)]
pub struct HandleMap<T>
where
    T: Copy + Eq + Hash,
{
    handles: HashMap<EntityId, T>,
    rev: HashMap<T, EntityId>,
}

impl<T> HandleMap<T>
where
    T: Copy + Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
            rev: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: EntityId, handle: T) {
        self.handles.insert(id, handle);
        self.rev.insert(handle, id);
    }

    pub fn get(&self, id: EntityId) -> Option<T> {
        self.handles.get(&id).copied()
    }

    pub fn get2(&self, handle: T) -> Option<EntityId> {
        self.rev.get(&handle).copied()
    }

    pub fn remove(&mut self, id: EntityId) -> Option<T> {
        let handle = self.handles.remove(&id)?;
        self.rev.remove(&handle);
        Some(handle)
    }

    pub fn remove2(&mut self, handle: T) -> Option<EntityId> {
        let id = self.rev.remove(&handle)?;
        self.handles.remove(&id);
        Some(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        self.handles.values().copied()
    }
}
