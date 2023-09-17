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

    pub fn get(&self, entity: Entity) -> Option<Transform> {
        self.nodes.get(entity.0).copied()
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
            if self.parents.get(&Entity(key)).is_none() {
                transforms.insert(key, *transform);
            }

            if let Some(children) = self.children.get(&Entity(key)) {
                for child in children {
                    parents.insert(*child, key);
                }
            }
        }

        while !parents.is_empty() {
            for (child, parent) in parents.clone().iter() {
                if let Some(transform) = transforms.get(parent) {
                    let local_transform = self.nodes.get(child.0).unwrap();
                    parents.remove(child);

                    transforms.insert(child.0, transform.mul_transform(*local_transform));
                }
            }
        }

        for (key, transform) in transforms.into_iter() {
            *self.global_transform.get_mut(&Entity(key)).unwrap() = transform;
        }
    }

    pub fn children(&self, entity: Entity) -> Option<impl Iterator<Item = Entity> + '_> {
        self.children.get(&entity).map(|vec| vec.iter().copied())
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
