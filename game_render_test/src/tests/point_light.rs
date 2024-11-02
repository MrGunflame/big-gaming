use game_common::components::{Color, Transform};
use game_render::camera::{Camera, Projection};
use game_render::entities::Object;
use game_render::light::PointLight;
use game_render::mesh::Mesh;
use game_render::pbr::PbrMaterial;
use game_render::shape::Plane;
use glam::Vec3;

use crate::Harness;

pub(super) fn point_light() -> Harness {
    Harness::new(stringify!(point_light), |renderer, target| {
        renderer.scene.entities.cameras.insert(Camera {
            transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            target,
            projection: Projection {
                aspect_ratio: 1.0,
                fov: 90.0,
                near: 0.1,
                far: 1000.0,
            },
        });

        let plane = Mesh::from(Plane { size: 10.0 });
        let mesh = renderer.meshes.insert(plane);
        let material = renderer.materials.insert(PbrMaterial::default());

        renderer.scene.entities.objects.insert(Object {
            transform: Transform::default(),
            mesh,
            material,
        });

        renderer.scene.entities.point_lights.insert(PointLight {
            transform: Transform::from_translation(Vec3::new(0.0, 1.0, -5.0)),
            color: Color::WHITE,
            intensity: 50.0,
            radius: 100.0,
        });
    })
}
