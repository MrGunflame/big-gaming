use std::collections::HashMap;

use game_common::components::transform::Transform;
use slotmap::{DefaultKey, SlotMap};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(DefaultKey);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entity(Key);

#[derive(Clone, Debug)]
pub struct Hierarchy<T> {
    nodes: SlotMap<DefaultKey, T>,
    children: HashMap<Key, Vec<Key>>,
    parents: HashMap<Key, Key>,
}

impl<T> Hierarchy<T> {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::new(),
            children: HashMap::new(),
            parents: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn append(&mut self, parent: Option<Key>, node: T) -> Key {
        let key = Key(self.nodes.insert(node));

        if let Some(parent) = parent {
            debug_assert!(self.nodes.contains_key(parent.0));

            self.parents.insert(key, parent);
            self.children.entry(parent).or_default().push(key);
        }

        key
    }

    pub fn remove(&mut self, key: Key) {
        self.nodes.remove(key.0);

        if let Some(parent) = self.parents.remove(&key) {
            if let Some(children) = self.children.get_mut(&parent) {
                children.retain(|id| *id != key);
            }
        }

        if let Some(children) = self.children.remove(&key) {
            for c in children {
                self.remove(c);
            }
        }
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        self.nodes.get(key.0)
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        self.nodes.get_mut(key.0)
    }

    /// Removes all entities.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.children.clear();
        self.parents.clear();
    }

    pub fn contains_key(&self, key: Key) -> bool {
        self.nodes.contains_key(key.0)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, &T)> + '_ {
        self.nodes.iter().map(|(k, v)| (Key(k), v))
    }

    pub fn values(&self) -> impl Iterator<Item = &T> + '_ {
        self.nodes.values()
    }

    pub fn parent(&self, key: Key) -> Option<&T> {
        let parent = self.parents.get(&key)?;
        Some(self.nodes.get(parent.0).unwrap())
    }

    pub fn children(&self, parent: Key) -> Option<impl Iterator<Item = (Key, &T)> + '_> {
        let children = self.children.get(&parent)?;
        Some(children.iter().map(|key| {
            let node = self.nodes.get(key.0).unwrap();
            (*key, node)
        }))
    }

    /// Converts a `Hierarchy<T>` into a `Hierarchy<U>`, retaining the existing hierarchy.
    pub fn convert<U, F>(&self, mut f: F) -> Hierarchy<U>
    where
        F: FnMut(&T) -> U,
    {
        // FIXME: Actually we only need to create a new arena in the same
        // state, apply `F` on all elements and then reuse the existing keys
        // as `self.nodes`, but `SlotMap` doesn't allow us to do these things.
        // We can do this only we have our own `Arena` type.
        // For now we must manually recreate the parents/children maps.
        let mut nodes = SlotMap::with_capacity(self.nodes.len());
        let mut parents = HashMap::with_capacity(self.parents.len());
        let mut children = HashMap::with_capacity(self.children.len());

        let mut old_to_new_keys = HashMap::new();

        for (old_key, node) in &self.nodes {
            let new_key = nodes.insert(f(node));
            old_to_new_keys.insert(old_key, new_key);
        }

        for (old_key, old_parent) in &self.parents {
            let new_key = old_to_new_keys.get(&old_key.0).unwrap();
            let new_parent = old_to_new_keys.get(&old_parent.0).unwrap();
            parents.insert(Key(*new_key), Key(*new_parent));
        }

        for (old_key, old_children) in &self.children {
            let new_key = old_to_new_keys.get(&old_key.0).unwrap();
            let new_children = old_children
                .iter()
                .map(|k| Key(*old_to_new_keys.get(&k.0).unwrap()))
                .collect();

            children.insert(Key(*new_key), new_children);
        }

        Hierarchy {
            nodes,
            children,
            parents,
        }
    }
}

impl<T> Default for Hierarchy<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for Hierarchy<T> {
    fn from(value: T) -> Self {
        let mut hierarchy = Self::new();
        hierarchy.append(None, value);
        hierarchy
    }
}

#[derive(Clone, Debug, Default)]
pub struct TransformHierarchy {
    hierarchy: Hierarchy<Transform>,
    global_transform: HashMap<Entity, Transform>,
}

impl TransformHierarchy {
    pub fn new() -> Self {
        Self {
            hierarchy: Hierarchy::new(),
            global_transform: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.hierarchy.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn append(&mut self, parent: Option<Entity>, transform: Transform) -> Entity {
        let key = self.hierarchy.append(parent.map(|e| e.0), transform);
        self.global_transform.insert(Entity(key), transform);
        Entity(key)
    }

    pub fn remove(&mut self, entity: Entity) {
        self.hierarchy.remove(entity.0);
        self.global_transform.remove(&entity);
    }

    pub fn get(&self, entity: Entity) -> Option<Transform> {
        self.hierarchy.get(entity.0).copied()
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut Transform> {
        self.hierarchy.get_mut(entity.0)
    }

    pub fn set(&mut self, entity: Entity, transform: Transform) {
        if let Some(t) = self.hierarchy.get_mut(entity.0) {
            *t = transform;
        }
    }

    pub fn compute_transform(&mut self) {
        // FIXME: This is a 1:1 copy from the old ECS implementation that is
        // still extreamly inefficient.

        let mut transforms = HashMap::new();
        let mut parents = HashMap::new();

        for (key, transform) in &self.hierarchy.nodes {
            if self.hierarchy.parents.get(&Key(key)).is_none() {
                transforms.insert(key, *transform);
            }

            if let Some(children) = self.hierarchy.children.get(&Key(key)) {
                for child in children {
                    parents.insert(*child, key);
                }
            }
        }

        while !parents.is_empty() {
            for (child, parent) in parents.clone().iter() {
                if let Some(transform) = transforms.get(parent) {
                    let local_transform = self.hierarchy.nodes.get(child.0).unwrap();
                    parents.remove(child);

                    transforms.insert(child.0, transform.mul_transform(*local_transform));
                }
            }
        }

        for (key, transform) in transforms.into_iter() {
            *self.global_transform.get_mut(&Entity(Key(key))).unwrap() = transform;
        }
    }

    /// Returns an iterator over all entities with an updated transform.
    pub fn iter_changed_global_transform(&self) -> impl Iterator<Item = (Entity, Transform)> + '_ {
        self.global_transform.iter().map(|(k, v)| (*k, *v))
    }

    pub fn children(&self, entity: Entity) -> Option<impl Iterator<Item = Entity> + '_> {
        self.hierarchy
            .children
            .get(&entity.0)
            .map(|vec| vec.iter().map(|k| Entity(*k)))
    }

    /// Removes all entities.
    pub fn clear(&mut self) {
        self.hierarchy.nodes.clear();
        self.hierarchy.children.clear();
        self.hierarchy.parents.clear();
        self.global_transform.clear();
    }

    pub fn exists(&self, entity: Entity) -> bool {
        self.hierarchy.contains_key(entity.0)
    }
}

#[cfg(test)]
mod tests {
    use game_common::components::transform::Transform;
    use glam::Vec3;

    use super::TransformHierarchy;

    #[test]
    fn hierarchy_compute_transform_deep() {
        let mut hierarchy = TransformHierarchy::new();

        let mut entities = Vec::new();
        let mut last_entity = None;

        for _ in 0..5 {
            let entity = hierarchy.append(last_entity, Transform::default());

            entities.push(entity);
            last_entity = Some(entity);
        }

        for entity in &entities {
            let transform = Transform::from_translation(Vec3::new(1.0, 0.0, 0.0));

            hierarchy.set(*entity, transform);
        }

        hierarchy.compute_transform();

        for (index, entity) in entities.iter().enumerate() {
            let transform = *hierarchy.global_transform.get(entity).unwrap();

            assert_eq!(
                transform,
                Transform::from_translation(Vec3::new((index + 1) as f32, 0.0, 0.0))
            );
        }
    }
}
