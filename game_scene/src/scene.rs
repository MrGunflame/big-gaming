use bevy_ecs::prelude::{Bundle, Entity};
use bevy_ecs::system::{Commands, Query, Res, ResMut};
use game_asset::Assets;
use game_common::bundles::TransformBundle;
use game_common::components::transform::Transform;
use game_core::hierarchy::Children;
use game_render::color::Color;
use game_render::mesh::Mesh;
use game_render::pbr::{PbrBundle, PbrMaterial};
use game_render::shape;

use crate::{SceneHandle, Scenes};

pub(crate) fn spawn_scene(
    mut commands: Commands,
    scenes: Res<Scenes>,
    entities: Query<(Entity, &SceneHandle, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PbrMaterial>>,
) {
    for (entity, scene, transform) in &entities {
        let scene = match scenes.get(scene) {
            Some(scene) => scene,
            None => continue,
        };

        let mut children = Children::new();

        for node in &scene.nodes {
            let id = commands
                .spawn(PbrBundle {
                    mesh: node.mesh.clone(),
                    material: node.material.clone(),
                    transform: TransformBundle {
                        transform: node.transform,
                        ..Default::default()
                    },
                })
                .id();

            children.push(id);
        }

        // Local Coordinate axes for debugging
        for (mesh, color) in [
            (
                shape::Box {
                    min_x: 0.0,
                    max_x: 2.0,
                    min_y: -0.1,
                    max_y: 0.1,
                    min_z: -0.1,
                    max_z: 0.1,
                },
                Color::RED,
            ),
            (
                shape::Box {
                    min_x: -0.1,
                    max_x: 0.1,
                    min_y: 0.0,
                    max_y: 2.0,
                    min_z: -0.1,
                    max_z: 0.1,
                },
                Color::GREEN,
            ),
            (
                shape::Box {
                    min_x: -0.1,
                    max_x: 0.1,
                    min_y: -0.1,
                    max_y: 0.1,
                    min_z: 0.0,
                    max_z: 2.0,
                },
                Color::BLUE,
            ),
        ] {
            let id = commands
                .spawn(PbrBundle {
                    mesh: meshes.insert(mesh.into()),
                    material: materials.insert(PbrMaterial {
                        base_color: color,
                        ..Default::default()
                    }),
                    transform: TransformBundle::default(),
                })
                .id();

            children.push(id);
        }

        commands
            .entity(entity)
            .remove::<SceneHandle>()
            .insert(children);
    }
}

#[derive(Clone, Debug, Bundle)]
pub struct SceneBundle {
    pub scene: SceneHandle,
    #[bundle]
    pub transform: TransformBundle,
}
