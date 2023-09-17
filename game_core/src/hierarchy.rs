use std::collections::HashMap;

use game_common::components::transform::Transform;
use slotmap::{DefaultKey, SlotMap};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entity(DefaultKey);

#[derive(Clone, Debug, Default)]
pub struct TransformHierarchy {
    nodes: SlotMap<DefaultKey, Transform>,
    children: HashMap<Entity, Vec<Entity>>,
    parents: HashMap<Entity, Entity>,
    global_transform: HashMap<Entity, Transform>,
}

impl TransformHierarchy {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::new(),
            children: HashMap::new(),
            parents: HashMap::new(),
            global_transform: HashMap::new(),
        }
    }

    pub fn append(&mut self, parent: Option<Entity>, transform: Transform) -> Entity {
        let key = Entity(self.nodes.insert(transform));
        self.global_transform.insert(key, transform);

        if let Some(parent) = parent {
            debug_assert!(self.nodes.contains_key(parent.0));

            self.parents.insert(key, parent);
            self.children.entry(parent).or_default().push(key);
        }

        key
    }

    pub fn remove(&mut self, entity: Entity) {
        self.nodes.remove(entity.0);

        if let Some(parent) = self.parents.remove(&entity) {
            if let Some(children) = self.children.get_mut(&parent) {
                children.retain(|id| *id != entity);
            }
        }

        if let Some(children) = self.children.remove(&entity) {
            for c in children {
                self.remove(c);
            }
        }
    }

    pub fn set(&mut self, entity: Entity, transform: Transform) {
        if let Some(t) = self.nodes.get_mut(entity.0) {
            *t = transform;
        }
    }

    /// Returns an iterator over all entities with an updated transform.
    pub fn iter_changed_transform(&self) -> impl Iterator<Item = (Entity, Transform)> + '_ {
        self.nodes.iter().map(|(k, v)| (Entity(k), *v))
    }

    pub fn compute_transform(&mut self) {
        // FIXME: This is a 1:1 copy from the old ECS implementation that is
        // still extreamly inefficient.

        let mut transforms = HashMap::new();
        let mut parents = HashMap::new();

        for (key, transform) in &self.nodes {
            transforms.insert(key, *transform);

            if let Some(children) = self.children.get(&Entity(key)) {
                for child in children {
                    parents.insert(*child, key);
                }
            }
        }

        while !parents.is_empty() {
            for (child, parent) in parents.clone().iter() {
                if let Some(transform) = transforms.get(parent) {
                    let local_transform = transforms.get(&child.0).unwrap();

                    transforms.insert(child.0, transform.mul_transform(*local_transform));
                }
            }
        }

        for (key, transform) in transforms.into_iter() {
            *self.global_transform.get_mut(&Entity(key)).unwrap() = transform;
        }
    }
}
