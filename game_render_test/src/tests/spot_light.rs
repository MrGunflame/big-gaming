use std::f32::consts::PI;

use game_common::components::{Color, Transform};
use game_core_pipeline::camera::{Camera, Projection};
use game_core_pipeline::entities::Object;
use game_core_pipeline::lights::{Light, SpotLight};
use game_core_pipeline::material::StandardMaterial;
use game_render::mesh::Mesh;
use game_render::shape::Plane;
use glam::Vec3;

use crate::Harness;

pub(super) fn spot_light() -> Harness {
    Harness::new(stringify!(spot_light), |entities, scene, target| {
        let camera = entities.create_camera(Camera {
            transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            target,
            projection: Projection {
                aspect_ratio: 1.0,
                fov: 90.0,
                near: 1.0,
                far: 1000.0,
            },
            scene: scene.clone(),
        });

        let plane = Mesh::from(Plane { size: 10.0 });
        let mesh = entities.create_mesh(plane);
        let material = entities.create_material(StandardMaterial::default());

        let object = entities.create_object(Object {
            transform: Transform::default(),
            mesh,
            material,
            scene: scene.clone(),
        });

        let light = entities.create_light(Light::Spot(SpotLight {
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
            scene,
        }));

        // TODO: Instead of leaking the handles directly
        // we should add a `forget` or `leak` function to
        // the handles which releases the handle cleanly
        // without sending a signal to destroy the referenced
        // entity.
        core::mem::forget(camera);
        core::mem::forget(object);
        core::mem::forget(light);
    })
}
