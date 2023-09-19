use std::collections::HashMap;

use game_common::components::transform::Transform;
use game_core::hierarchy::{Entity, TransformHierarchy};
use game_render::color::Color;
use game_render::entities::{Object, ObjectId};
use game_render::pbr::PbrMaterial;
use game_render::{shape, Renderer};
use game_tracing::trace_span;

use crate::Scene;

pub(crate) fn spawn_scene(
    scene: &Scene,
    renderer: &mut Renderer,
    hierarchy: &mut TransformHierarchy,
    nodes: &mut HashMap<Entity, ObjectId>,
) -> Entity {
    let _span = trace_span!("spawn_scene").entered();

    let root = hierarchy.append(None, Transform::default());

    for node in &scene.nodes {
        let key = hierarchy.append(Some(root), node.transform);

        let id = renderer.entities.objects.insert(Object {
            transform: Transform::default(),
            mesh: node.mesh.clone(),
            material: node.material.clone(),
        });

        nodes.insert(key, id);
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
        renderer.entities.objects.insert(Object {
            transform: Default::default(),
            mesh: renderer.meshes.insert(mesh.into()),
            material: renderer.materials.insert(PbrMaterial {
                base_color: color,
                ..Default::default()
            }),
        });
    }

    root
}
