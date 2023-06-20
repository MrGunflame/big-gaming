use bevy_ecs::prelude::{Bundle, Entity};
use bevy_ecs::system::{Commands, Query, Res};
use game_common::bundles::TransformBundle;
use game_common::components::transform::Transform;
use game_render::pbr::PbrBundle;

use crate::{SceneHandle, Scenes};

pub(crate) fn spawn_scene(
    mut commands: Commands,
    scenes: Res<Scenes>,
    entities: Query<(Entity, &SceneHandle, &Transform)>,
) {
    for (entity, scene, transform) in &entities {
        let scene = scenes.get(scene).unwrap();

        for node in &scene.nodes {
            commands.spawn(PbrBundle {
                mesh: node.mesh.clone(),
                material: node.material.clone(),
                transform: TransformBundle {
                    transform: *transform * node.transform,
                    ..Default::default()
                },
            });
        }

        commands.entity(entity).remove::<SceneHandle>();
    }
}

#[derive(Clone, Debug, Bundle)]
pub struct SceneBundle {
    pub scene: SceneHandle,
    #[bundle]
    pub transform: TransformBundle,
}
