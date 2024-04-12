use std::collections::HashMap;

use game_wasm::components::builtin::{GlobalTransform, Transform};
use game_wasm::hierarchy::Children;

use crate::entity::EntityId;

use super::World;

pub fn update_global_transform(world: &mut World) {
    let mut transforms = HashMap::new();
    let mut parents = HashMap::new();

    for (entity, transform) in world.query::<Transform>() {
        transforms.insert(entity, transform);
    }

    for (entity, children) in world.query::<Children>() {
        for child in children.get() {
            parents.insert(EntityId::from_raw(child.into_raw()), entity);
        }
    }

    while !parents.is_empty() {
        for (child, parent) in parents.clone().iter() {
            if let Some(transform) = transforms.get(parent) {
                let local_transform = transforms.get(child).unwrap();
                parents.remove(child);

                transforms.insert(*child, transform.mul_transform(*local_transform));
            }
        }
    }

    for (entity, transform) in transforms {
        world.insert_typed(entity, GlobalTransform(transform));
    }
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
