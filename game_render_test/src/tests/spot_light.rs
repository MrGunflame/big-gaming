use std::f32::consts::PI;

use game_common::components::{Color, Transform};
use game_render::camera::{Camera, Projection};
use game_render::entities::Object;
use game_render::light::SpotLight;
use game_render::mesh::Mesh;
use game_render::pbr::PbrMaterial;
use game_render::shape::Plane;
use glam::{Quat, Vec3};

use crate::Harness;

pub(super) fn spot_light() -> Harness {
    Harness::new(stringify!(spot_light), |renderer, target| {
        renderer.entities.cameras.insert(Camera {
            transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            target,
            projection: Projection {
                aspect_ratio: 1.0,
                fov: 90.0,
                near: 1.0,
                far: 1000.0,
            },
        });

        let plane = Mesh::from(Plane { size: 10.0 });
        let mesh = renderer.meshes.insert(plane);
        let material = renderer.materials.insert(PbrMaterial::default());

        renderer.entities.objects.insert(Object {
            transform: Transform::default(),
            mesh,
            material,
        });

        renderer.entities.spot_lights.insert(SpotLight {
            transform: Transform {
                translation: Vec3::new(0.0, 1.0, -5.0),
                // rotation: Quat::from_axis_angle(Vec3::Y, PI / 2.0),
                ..Default::default()
            },
            color: Color::WHITE,
            intensity: 50.0,
            radius: 100.0,
            inner_cutoff: PI / 8.0,
            outer_cutoff: PI / 4.0,
        });
    })
}
