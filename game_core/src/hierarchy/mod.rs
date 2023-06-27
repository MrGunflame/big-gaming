use std::collections::{HashMap, HashSet, VecDeque};

use bevy_app::{App, CoreSet, Plugin};
use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::query::{Added, Changed, Or};
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::schedule::{IntoSystemConfig, SystemSet};
use bevy_ecs::system::{Commands, Query, ResMut, Resource};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, SystemSet)]
pub struct HierarchySet;

pub struct HierarchyPlugin;

impl Plugin for HierarchyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EntityChildren::default());
        app.add_system(
            update_children
                .in_base_set(CoreSet::PostUpdate)
                .in_set(HierarchySet),
        );
        app.add_system(
            despawn_children
                .in_base_set(CoreSet::PostUpdate)
                .in_set(HierarchySet)
                .after(update_children),
        );
    }
}

#[derive(Default, Resource)]
struct EntityChildren {
    entities: HashMap<Entity, HashSet<Entity>>,
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

fn despawn_children(
    mut commands: Commands,
    mut childs: ResMut<EntityChildren>,
    mut entities: RemovedComponents<Children>,
) {
    let mut queued = VecDeque::new();

    for entity in entities.iter() {
        queued.push_back(entity);
    }

    while let Some(entity) = queued.pop_front() {
        if let Some(mut cmds) = commands.get_entity(entity) {
            cmds.despawn();
        }

        if let Some(children) = childs.entities.remove(&entity) {
            queued.extend(children);
        }
    }
}

fn update_children(
    mut childs: ResMut<EntityChildren>,
    entities: Query<(Entity, &Children), Or<(Changed<Children>, Added<Children>)>>,
) {
    for (entity, children) in &entities {
        *childs.entities.entry(entity).or_default() = children.entities.clone();
    }
}
