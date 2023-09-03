use game_render::color::Color;
use game_render::entities::Object;
use game_render::pbr::PbrMaterial;
use game_render::{shape, RenderState};

use crate::Scene;

pub(crate) fn spawn_scene(scene: &Scene, renderer: &mut RenderState) {
    for node in &scene.nodes {
        renderer.entities.push_object(Object {
            transform: Default::default(),
            mesh: node.mesh.clone(),
            material: node.material.clone(),
        });
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
        renderer.entities.push_object(Object {
            transform: Default::default(),
            mesh: renderer.meshes.insert(mesh.into()),
            material: renderer.materials.insert(PbrMaterial {
                base_color: color,
                ..Default::default()
            }),
        });
    }
}
