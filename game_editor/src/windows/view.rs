//! An immutable view of a scene.

use std::f32::consts::PI;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Commands;
use game_render::camera::{Camera, CameraBundle, RenderTarget, Transform};
use game_render::material::{Material, MaterialMeshBundle};
use game_render::shape;
use glam::{Quat, Vec3};

pub fn spawn_view_window(commands: &mut Commands, id: Entity) {
    commands.spawn(CameraBundle {
        camera: Camera {
            projection: Default::default(),
            target: RenderTarget::Window(id),
        },
        transform: Transform::default(),
    });

    // commands.spawn(MaterialMeshBundle {
    //     mesh: shape::Box {
    //         min_x: -0.5,
    //         max_x: 0.5,
    //         min_y: -0.5,
    //         max_y: 0.5,
    //         min_z: -0.5,
    //         max_z: 0.5,
    //     }
    //     .into(),
    //     material: Material::default(),
    //     computed_material: Default::default(),
    //     computed_mesh: Default::default(),
    // });

    let img = image::io::Reader::open("../assets/Baker.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();

    commands
        .spawn(MaterialMeshBundle {
            mesh: shape::Box {
                min_x: -0.5,
                max_x: 0.5,
                min_y: -0.5,
                max_y: 0.5,
                min_z: -0.5,
                max_z: 0.5,
            }
            .into(),
            material: Material {
                color: [1.0, 0.0, 0.0, 1.0],
                color_texture: img.clone(),
            },
            computed_material: Default::default(),
            computed_mesh: Default::default(),
        })
        .insert(Transform {
            translation: Vec3::new(0.0, 1.0, -5.0),
            rotation: Quat::from_axis_angle(Vec3::Y, PI / 3.0),
            ..Default::default()
        });
}
