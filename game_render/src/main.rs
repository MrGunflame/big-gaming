use std::f32::consts::PI;

use bevy_app::App;
use bevy_ecs::prelude::EventReader;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query};
use game_common::components::transform::Transform;
use game_input::keyboard::KeyboardInput;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::material::{Material, MaterialMeshBundle};
use game_render::{shape, RenderPlugin};
use game_window::events::VirtualKeyCode;
use game_window::Window;
use glam::{Quat, Vec3};

fn main() {
    let mut app = App::new();
    app.add_plugin(RenderPlugin);
    app.add_startup_system(setup);
    app.add_system(move_camera);

    app.run();
}

fn setup(mut cmds: Commands) {
    let id = cmds
        .spawn(Window {
            title: "test".to_owned(),
        })
        .id();

    cmds.spawn(Camera {
        target: RenderTarget::Window(id),
        projection: Projection::default(),
    })
    .insert(Transform::default());

    let img = image::io::Reader::open("../assets/Baker.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();

    cmds.spawn(MaterialMeshBundle {
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

    cmds.spawn(MaterialMeshBundle {
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
            color: [1.0, 1.0, 1.0, 1.0],
            color_texture: img,
        },
        computed_material: Default::default(),
        computed_mesh: Default::default(),
    })
    .insert(Transform {
        translation: Vec3::new(1.0, -0.5, -4.0),
        ..Default::default()
    });

    // cmds.spawn(MaterialMeshBundle {
    //     mesh: shape::Plane { size: 100.0 }.into(),
    // })
    // .insert(Transform {
    //     translation: Vec3::new(0.0, -5.0, 0.0),
    //     ..Default::default()
    // });
}

fn move_camera(
    mut events: EventReader<KeyboardInput>,
    mut cameras: Query<(&mut Transform), With<Camera>>,
) {
    let mut camera = cameras.single_mut();

    for event in events.iter() {
        match event.key_code {
            Some(VirtualKeyCode::W) => {
                let rot = camera.rotation * -Vec3::Z;
                camera.translation += rot;
            }
            Some(VirtualKeyCode::S) => {
                let rot = camera.rotation * -Vec3::Z;
                camera.translation -= rot;
            }
            Some(VirtualKeyCode::A) => {
                let rot = (camera.rotation * Quat::from_axis_angle(Vec3::Y, 90.0f32.to_radians()))
                    * -Vec3::Z;
                camera.translation += rot;
            }
            _ => (),
        }
    }
}
