//! An immutable view of a scene.

use bevy_ecs::prelude::{Component, EventReader, Res};
use bevy_ecs::query::{Changed, With};
use bevy_ecs::system::{Commands, Query};
use bitflags::bitflags;
use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButton, MouseButtonInput, MouseMotion, MouseWheel};
use game_input::ButtonState;
use game_render::camera::{Camera, CameraBundle, RenderTarget, Transform};
use game_render::material::{Material, MaterialMeshBundle};
use game_render::shape;
use game_ui::cursor::Cursor;
use game_window::events::{CursorLeft, VirtualKeyCode};
use game_window::Window;
use glam::{Quat, Vec3};

pub fn spawn_view_window(commands: &mut Commands) {
    let id = commands
        .spawn(Window {
            title: "test".to_owned(),
        })
        .insert(ViewWindowState::default())
        .id();

    commands
        .spawn(CameraBundle {
            camera: Camera {
                projection: Default::default(),
                target: RenderTarget::Window(id),
            },
            transform: Transform::default(),
        })
        .insert(ViewCamera);

    let mesh = &game_gltf::GltfData::open("../assets/pistol.glb")
        .unwrap()
        .meshes()
        .unwrap()[0]
        .0;

    commands
        .spawn(MaterialMeshBundle {
            mesh: mesh.clone(),
            material: Material {
                color: [1.0, 0.0, 0.0, 1.0],
                ..Default::default()
            },
            computed_material: Default::default(),
            computed_mesh: Default::default(),
        })
        .insert(Transform {
            translation: Vec3::new(0.0, 1.0, -5.0),
            // rotation: Quat::from_axis_angle(Vec3::Y, PI / 3.0),
            ..Default::default()
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

    // commands
    //     .spawn(MaterialMeshBundle {
    //         mesh: shape::Box {
    //             min_x: -0.5,
    //             max_x: 0.5,
    //             min_y: -0.5,
    //             max_y: 0.5,
    //             min_z: -0.5,
    //             max_z: 0.5,
    //         }
    //         .into(),
    //         material: Material {
    //             color: [1.0, 0.0, 0.0, 1.0],
    //             color_texture: img.clone(),
    //         },
    //         computed_material: Default::default(),
    //         computed_mesh: Default::default(),
    //     })
    //     .insert(Transform {
    //         translation: Vec3::new(0.0, 1.0, -5.0),
    //         // rotation: Quat::from_axis_angle(Vec3::Y, PI / 3.0),
    //         ..Default::default()
    //     });

    commands
        .spawn(MaterialMeshBundle {
            mesh: shape::Box {
                min_x: -0.1,
                max_x: 0.1,
                min_y: -0.1,
                max_y: 0.1,
                min_z: -0.1,
                max_z: 0.1,
            }
            .into(),
            material: Material {
                color: [1.0, 1.0, 1.0, 1.0],
                color_texture: img.clone(),
            },
            computed_material: Default::default(),
            computed_mesh: Default::default(),
        })
        .insert(Transform::default())
        .insert(OriginMarker);

    for (mesh, color) in [
        (
            shape::Box {
                min_x: 0.0,
                max_x: 1.0,
                min_y: -0.1,
                max_y: 0.1,
                min_z: -0.1,
                max_z: 0.1,
            },
            [1.0, 0.0, 0.0, 1.0],
        ),
        (
            shape::Box {
                min_x: -0.1,
                max_x: 0.1,
                min_y: 0.0,
                max_y: 1.0,
                min_z: -0.1,
                max_z: 0.1,
            },
            [0.0, 1.0, 0.0, 1.0],
        ),
        (
            shape::Box {
                min_x: -0.1,
                max_x: 0.1,
                min_y: -0.1,
                max_y: 0.1,
                min_z: 0.0,
                max_z: 1.0,
            },
            [0.0, 0.0, 1.0, 1.0],
        ),
    ] {
        commands
            .spawn(MaterialMeshBundle {
                mesh: mesh.into(),
                material: Material {
                    color,
                    ..Default::default()
                },
                computed_material: Default::default(),
                computed_mesh: Default::default(),
            })
            .insert(Transform::default());
    }
}

/// state attached to windows with a view.
#[derive(Clone, Debug, Default, Component)]
pub struct ViewWindowState {
    origin: Vec3,
    mode: Mode,
}

#[derive(Copy, Clone, Debug, Component)]
pub struct ViewCamera;

#[derive(Copy, Clone, Debug, Component)]
pub struct OriginMarker;

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    struct Mode: u8 {
        const MIDDLE = 0b01;
        const SHIFT = 0b10;
    }
}

impl Mode {
    const NONE: Self = Self::from_bits_truncate(0b00);
    const ROTATE: Self = Self::from_bits_truncate(0b01);
    const TRANSLATE: Self = Self::from_bits_truncate(0b11);
}

pub fn reset_state_on_cursor_leave(
    mut windows: Query<&mut ViewWindowState>,
    mut events: EventReader<CursorLeft>,
) {
    for event in events.iter() {
        let Ok(mut state) = windows.get_mut(event.window) else {
            continue;
        };

        state.mode = Mode::NONE;
    }
}

pub fn zoom_scene(
    cursor: Res<Cursor>,
    windows: Query<&ViewWindowState>,
    mut cameras: Query<(&mut Transform, &Camera), With<ViewCamera>>,
    mut events: EventReader<MouseWheel>,
) {
    let Some(window) = cursor.window() else {
        events.clear();
        return;
    };

    let Ok(state) = windows.get(window) else {
        events.clear();
        return;
    };

    if state.mode != Mode::NONE {
        events.clear();
        return;
    }

    for event in events.iter() {
        for (mut transform, _) in cameras
            .iter_mut()
            .filter(|(_, cam)| cam.target == RenderTarget::Window(window))
        {
            let dir = transform.rotation * -Vec3::Z;

            transform.translation -= dir * event.y * 0.05;
        }
    }
}

pub fn update_camera_mode(
    cursor: Res<Cursor>,
    mut windows: Query<&mut ViewWindowState>,
    mut mouse: EventReader<MouseButtonInput>,
    mut keyboard: EventReader<KeyboardInput>,
) {
    let Some(window) = cursor.window() else {
        mouse.clear();
        keyboard.clear();
        return;
    };

    let Ok(mut state) = windows.get_mut(window) else {
        mouse.clear();
        keyboard.clear();
        return;
    };

    for event in mouse.iter() {
        if event.button != MouseButton::Middle {
            continue;
        }

        match event.state {
            ButtonState::Pressed => state.mode |= Mode::MIDDLE,
            ButtonState::Released => state.mode &= !Mode::MIDDLE,
        };
    }

    for event in keyboard.iter() {
        if event.key_code != Some(VirtualKeyCode::LShift) {
            continue;
        }

        match event.state {
            ButtonState::Pressed => state.mode |= Mode::SHIFT,
            ButtonState::Released => state.mode &= !Mode::SHIFT,
        }
    }
}

pub fn update_view_camera(
    cursor: Res<Cursor>,
    mut windows: Query<&mut ViewWindowState>,
    mut cameras: Query<(&mut Transform, &Camera), With<ViewCamera>>,
    mut events: EventReader<MouseMotion>,
) {
    let Some(window) = cursor.window() else {
        events.clear();
        return;
    };

    let Ok(mut state) = windows.get_mut(window) else {
        events.clear();
        return;
    };

    match state.mode {
        mode if mode == Mode::TRANSLATE => {
            for event in events.iter() {
                let x = event.delta.x * 0.01;
                let y = -event.delta.y * 0.01;

                for (mut transform, _) in cameras
                    .iter_mut()
                    .filter(|(_, cam)| cam.target == RenderTarget::Window(window))
                {
                    let mut distance = (transform.rotation * Vec3::X) * x;
                    distance += (transform.rotation * Vec3::Y) * y;

                    transform.translation += distance;
                    state.origin += distance;
                }
            }
        }
        mode if mode == Mode::ROTATE => {
            for event in events.iter() {
                let x = event.delta.x * 0.01;
                let y = event.delta.y * 0.01;

                for (mut transform, _) in cameras
                    .iter_mut()
                    .filter(|(_, cam)| cam.target == RenderTarget::Window(window))
                {
                    // // Rotate around origin with a constant distance.
                    let distance = (transform.translation - state.origin).length().abs();

                    let q1 = Quat::from_axis_angle(Vec3::Y, -x);
                    let q2 = Quat::from_axis_angle(Vec3::X, -y);

                    transform.rotation = q1 * transform.rotation;
                    transform.rotation = transform.rotation * q2;

                    // Renormalize quat due to FP error creep.
                    if transform.rotation.is_normalized() {
                        transform.rotation = transform.rotation.normalize();
                    }

                    // FIXME: FP error creep means that distance will very slowly grow
                    // over time. Storing the radius instead of computing the distance
                    // would fix this.
                    transform.translation =
                        state.origin + transform.rotation * Vec3::new(0.0, 0.0, distance);
                }
            }
        }
        _ => (),
    }
}

pub fn update_origin(
    mut windows: Query<&ViewWindowState, Changed<ViewWindowState>>,
    mut entities: Query<&mut Transform, With<OriginMarker>>,
) {
    for window in &mut windows {
        for mut transform in &mut entities {
            transform.translation = window.origin;
        }
    }
}
