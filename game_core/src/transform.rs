use std::collections::HashMap;

use bevy_app::{CoreSet, Plugin};
use bevy_ecs::prelude::Entity;
use bevy_ecs::schedule::{IntoSystemConfig, SystemSet};
use bevy_ecs::system::Query;
use game_common::components::transform::{GlobalTransform, Transform};

use crate::hierarchy::{Children, HierarchySet};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, SystemSet)]
pub struct TransformSet;

pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_system(
            update_global_transform
                .in_base_set(CoreSet::PostUpdate)
                .in_set(TransformSet)
                .after(HierarchySet),
        );
    }
}

pub fn update_global_transform(
    mut entities: Query<(Entity, &Transform, &mut GlobalTransform, Option<&Children>)>,
) {
    let mut transforms = HashMap::new();
    // Key requires Value to be computed first, i.e. extract parents.
    let mut parents = HashMap::new();

    for (entity, transform, _, children) in &entities {
        debug_assert!(
            transform.is_valid(),
            "Entity {:?} with invalid Transform: {:?}",
            entity,
            transform
        );

        transforms.insert(entity, *transform);

        if let Some(children) = children {
            for child in children.iter() {
                parents.insert(child, entity);
            }
        }
    }

    while !parents.is_empty() {
        for (child, parent) in parents.clone().iter() {
            // Parent has its transform computed, we can computed
            // child transform.
            if let Some(transform) = transforms.get(&parent) {
                let local_transform = transforms.get(child).unwrap();

                transforms.insert(*child, transform.mul_transform(*local_transform));
                parents.remove(child);
            }
        }
    }

    for (entity, transform) in transforms.into_iter() {
        let (_, _, mut global, _) = entities.get_mut(entity).unwrap();

        debug_assert!(
            transform.is_valid(),
            "Entity {:?} with invalid GlobalTransform: {:?}",
            entity,
            transform
        );

        global.0 = transform;
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_ecs::world::World;
    use game_common::bundles::TransformBundle;
    use game_common::components::transform::{GlobalTransform, Transform};
    use glam::Vec3;

    use crate::hierarchy::Children;

    use super::TransformPlugin;

    #[test]
    fn global_transform_no_children() {
        let mut app = App::new();
        app.add_plugin(TransformPlugin);

        let ids: Vec<_> = (0..3)
            .map(|i| {
                app.world
                    .spawn(TransformBundle {
                        transform: Transform {
                            translation: Vec3::splat(i as f32),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .id()
            })
            .collect();

        app.update();

        for (i, entity) in ids.iter().enumerate() {
            let transform = app.world.get::<GlobalTransform>(*entity).unwrap();
            assert_eq!(transform.0.translation, Vec3::splat(i as f32));
        }
    }

    #[test]
    fn global_transform_children_depth1() {
        let mut app = App::new();
        app.add_plugin(TransformPlugin);

        let spawn_children = |world: &mut World| world.spawn(TransformBundle::default()).id();

        let entities: Vec<_> = (0..3)
            .map(|i| {
                let children: Vec<_> = (0..3).map(|_| spawn_children(&mut app.world)).collect();

                let id = app
                    .world
                    .spawn(TransformBundle {
                        transform: Transform {
                            translation: Vec3::splat(i as f32),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .insert(Children::from_iter(&children))
                    .id();

                (id, children)
            })
            .collect();

        app.update();

        for (i, (entity, children)) in entities.into_iter().enumerate() {
            let transform = app.world.get::<GlobalTransform>(entity).unwrap();
            assert_eq!(transform.0.translation, Vec3::splat(i as f32));

            for child in children {
                let transform = app.world.get::<GlobalTransform>(child).unwrap();
                assert_eq!(transform.0.translation, Vec3::splat(i as f32));
            }
        }
    }
}
