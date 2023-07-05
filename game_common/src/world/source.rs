//! Streaming source
//!
use std::collections::HashMap;

use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;

use super::CellId;

/// An entity that (un)loads cells as it moves.
#[derive(Clone, Debug, Component)]
pub struct StreamingSource {
    pub state: StreamingState,
    /// The size of the area being loaded around this source.
    ///
    /// A `distance` of `0` also corresponds to only the cell that the entity resides in. Defaults
    /// to `0`.
    pub distance: u32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum StreamingState {
    /// The `StreamingSource` was just created.
    ///
    /// This signals the level loader that it should also load all cells which are already
    /// occupied by the current source.
    #[default]
    Create,
    /// The `StreamingSource` is normally active.
    ///
    /// This signals the level loader that it should only load/unload cells as the source moves.
    Active,
    /// The `StreamingSource` is being destroyed.
    ///
    /// This  signals the level loader that it should all cells currently loaded by the source.
    Destroy,
    Destroyed,
}

impl StreamingState {
    pub const fn is_create(self) -> bool {
        matches!(self, Self::Create)
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    pub const fn is_destroy(self) -> bool {
        matches!(self, Self::Destroy)
    }
}

impl StreamingSource {
    pub fn new() -> Self {
        Self {
            state: StreamingState::Create,
            distance: 0,
        }
    }
}

impl Default for StreamingSource {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Default, Resource)]
pub struct StreamingSources {
    sources: HashMap<CellId, u32>,
    loaded: Vec<CellId>,
    unloaded: Vec<CellId>,
}

impl StreamingSources {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            loaded: Vec::new(),
            unloaded: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn load(&mut self, id: CellId) {
        match self.sources.get_mut(&id) {
            Some(count) => *count += 1,
            None => {
                self.sources.insert(id, 1);
                self.loaded.push(id);
                // FIXME: In case the cell got already unloaded in this frame
                // it must be removed from self.unloaded.
            }
        }
    }

    pub fn unload(&mut self, id: CellId) {
        if let Some(count) = self.sources.get_mut(&id) {
            *count -= 1;
            if *count == 0 {
                self.sources.remove(&id);
                self.unloaded.push(id);
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = CellId> + '_ {
        self.sources.keys().cloned()
    }

    pub fn clear(&mut self) {
        self.loaded.clear();
        self.unloaded.clear();
    }

    pub fn loaded(&self) -> Loaded<'_> {
        Loaded {
            inner: &self.loaded,
        }
    }

    pub fn unloaded(&self) -> Loaded<'_> {
        Loaded {
            inner: &self.unloaded,
        }
    }
}

pub struct Loaded<'a> {
    inner: &'a [CellId],
}

impl<'a> Iterator for Loaded<'a> {
    type Item = CellId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (elem, rem) = self.inner.split_first()?;
        self.inner = rem;
        Some(*elem)
    }
}
