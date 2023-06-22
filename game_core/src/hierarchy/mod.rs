use std::collections::HashSet;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Component, Entity};

pub struct HierarchyPlugin;

impl Plugin for HierarchyPlugin {
    fn build(&self, app: &mut App) {}
}

#[derive(Clone, Debug, Component)]
pub struct Children {
    entities: HashSet<Entity>,
}

impl Children {
    pub fn new() -> Self {
        Self {
            entities: HashSet::new(),
        }
    }

    pub fn push(&mut self, entity: Entity) {
        self.entities.insert(entity);
    }

    pub fn remove(&mut self, entity: Entity) {
        self.entities.remove(&entity);
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            entities: self.entities.iter(),
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: HashSet::with_capacity(capacity),
        }
    }
}

impl FromIterator<Entity> for Children {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Entity>,
    {
        let iter = iter.into_iter();
        let (low, _) = iter.size_hint();

        let mut children = Self::with_capacity(low);

        for entity in iter.into_iter() {
            children.push(entity);
        }

        children
    }
}

impl<'a> FromIterator<&'a Entity> for Children {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a Entity>,
    {
        Self::from_iter(iter.into_iter().copied())
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    entities: std::collections::hash_set::Iter<'a, Entity>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.entities.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.entities.size_hint()
    }
}
