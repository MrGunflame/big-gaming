use game_asset::Assets;
use game_render::color::Color;
use game_render::entities::Object;
use game_render::mesh::Mesh;
use game_render::pbr::PbrMaterial;
use game_render::{shape, RenderState};

use crate::Scene;

pub(crate) fn spawn_scene(
    scene: &Scene,
    render_state: &mut RenderState,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<PbrMaterial>,
) {
    for node in &scene.nodes {
        render_state.entities.push_object(Object {
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
        render_state.entities.push_object(Object {
            transform: Default::default(),
            mesh: meshes.insert(mesh.into()),
            material: materials.insert(PbrMaterial {
                base_color: color,
                ..Default::default()
            }),
        });
    }
}
