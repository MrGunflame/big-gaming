use std::collections::{HashMap, VecDeque};

use game_wasm::components::builtin::{GlobalTransform, Transform};
use game_wasm::entity::EntityId;

use super::World;

/// Returns a list of entities with updated transform.
pub fn update_global_transform(world: &mut World) -> Vec<EntityId> {
    let mut transforms = HashMap::new();
    let mut parents = HashMap::new();

    for (entity, transform) in world.query::<Transform>() {
        transforms.insert(entity, transform);
    }

    let mut queued: VecDeque<_> = world.find_root_entities().into();

    while let Some(entity) = queued.pop_front() {
        match parents.get(&entity) {
            Some(parent) => {
                let parent_transform = transforms.get(&parent).unwrap();
                let children_transform = *transforms.get(&entity).unwrap();
                let transform = parent_transform.mul_transform(children_transform);
                transforms.insert(entity, transform);
            }
            None => (),
        };

        for child in world.children(entity) {
            parents.insert(child, entity);
            queued.push_back(child);
        }
    }

    let mut updated_entities = Vec::new();
    for (entity, transform) in transforms {
        if let Ok(GlobalTransform(old_transform)) = world.get_typed(entity) {
            if old_transform == transform {
                continue;
            }
        }

        updated_entities.push(entity);
        world.insert_typed(entity, GlobalTransform(transform));
    }

    updated_entities
}

#[cfg(test)]
mod tests {
    use game_wasm::components::builtin::{GlobalTransform, Transform};
    use game_wasm::entity::EntityId;
    use game_wasm::hierarchy::Children;
    use glam::Vec3;

    use crate::world::World;

    use super::update_global_transform;

    #[test]
    fn hierarchy_compute_transform() {
        let mut world = World::new();

        let child = world.spawn();
        world.insert_typed(child, Transform::from_translation(Vec3::splat(5.0)));

        let root = world.spawn();
        world.insert_typed(root, Transform::from_translation(Vec3::splat(10.0)));

        let mut children = Children::new();
        children.insert(EntityId::from_raw(child.into_raw()));
        world.insert_typed(root, children);

        update_global_transform(&mut world);

        assert_eq!(
            world
                .get_typed::<GlobalTransform>(child)
                .unwrap()
                .0
                .translation,
            Vec3::splat(15.0)
        );
        assert_eq!(
            world
                .get_typed::<GlobalTransform>(root)
                .unwrap()
                .0
                .translation,
            Vec3::splat(10.0)
        );
    }
}
