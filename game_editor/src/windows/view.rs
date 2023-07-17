//! An immutable view of a scene.

use std::f32::consts::PI;

use bevy_ecs::prelude::{Component, EventReader, Res};
use bevy_ecs::query::{Changed, With};
use bevy_ecs::system::{Commands, Query, ResMut};
use bitflags::bitflags;
use game_asset::Assets;
use game_common::bundles::TransformBundle;
use game_common::components::transform::Transform;
use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButton, MouseButtonInput, MouseMotion, MouseWheel};
use game_input::ButtonState;
use game_render::camera::{Camera, CameraBundle, RenderTarget};
use game_render::color::Color;
use game_render::light::{DirectionalLight, DirectionalLightBundle, PointLight, PointLightBundle};
use game_render::mesh::Mesh;
use game_render::pbr::{PbrBundle, PbrMaterial};
use game_render::shape;
use game_render::texture::{Image, Images, TextureFormat};
use game_scene::{SceneBundle, Scenes};
use game_ui::render::remap::remap;
use game_window::cursor::Cursor;
use game_window::events::{CursorLeft, VirtualKeyCode};
use game_window::{Window, WindowState};
use glam::{Quat, UVec2, Vec2, Vec3};

pub fn spawn_view_window(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<PbrMaterial>,
    images: &mut Images,
    scenes: &mut Scenes,
) {
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

    // let mesh = &game_gltf::GltfData::open("../assets/pistol.glb")
    //     .unwrap()
    //     .meshes()
    //     .unwrap()[0]
    //     .0;

    // commands
    //     .spawn(MaterialMeshBundle {
    //         mesh: mesh.clone(),
    //         material: Material {
    //             color: [1.0, 0.0, 0.0, 1.0],
    //             ..Default::default()
    //         },
    //         computed_material: Default::default(),
    //         computed_mesh: Default::default(),
    //     })
    //     .insert(Transform {
    //         translation: Vec3::new(0.0, 1.0, -5.0),
    //         // rotation: Quat::from_axis_angle(Vec3::Y, PI / 3.0),
    //         ..Default::default()
    //     });

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

    commands.spawn(PbrBundle {
        mesh: meshes.insert(shape::Plane { size: 100.0 }.into()),
        material: materials.insert(PbrMaterial {
            base_color: Color([1.0, 1.0, 1.0, 1.0]),
            ..Default::default()
        }),
        transform: TransformBundle {
            transform: Transform {
                translation: Vec3::new(0.0, -1.0, 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    });

    commands.spawn(DirectionalLightBundle {
        light: DirectionalLight {
            color: [1.0, 1.0, 1.0],
            illuminance: 1.0,
        },
        transform: TransformBundle {
            transform: Transform {
                translation: Vec3::new(1.0, 0.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        },
    });

    // commands.spawn(PointLightBundle {
    //     light: PointLight {
    //         color: Color::WHITE,
    //     },
    //     transform: TransformBundle {
    //         transform: Transform {
    //             translation: Vec3::new(0.0, 3.0, 3.0),
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     },
    // });

    // commands.spawn(DirectionalLightBundle {
    //     light: DirectionalLight {
    //         color: [0.1, 0.1, 1.0],
    //         illuminance: 1.0,
    //     },
    //     transform: TransformBundle {
    //         transform: Transform {
    //             translation: Vec3::new(-1.0, 0.0, 0.0),
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     },
    // });

    // commands.spawn(DirectionalLightBundle {
    //     light: DirectionalLight {
    //         color: [1.0, 0.1, 0.1],
    //         illuminance: 1.0,
    //     },
    //     transform: TransformBundle {
    //         transform: Transform {
    //             translation: Vec3::new(0.0, 0.0, 1.0),
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     },
    // });

    // commands.spawn(SceneBundle {
    //     scene: scenes
    //         .load("/home/robert/projects/gltf/glTF-Sample-Models/2.0/Sponza/glTF/Sponza.gltf"),
    //     transform: TransformBundle {
    //         transform: Transform {
    //             translation: Vec3::new(0.0, 0.0, 0.0),
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     },
    // });

    let metallic = image::io::Reader::open(
        "/home/robert/Downloads/rustediron1-alt2-bl/rustediron2_metallic.png",
    )
    .unwrap()
    .decode()
    .unwrap()
    .to_luma8();

    let roughness = image::io::Reader::open(
        "/home/robert/Downloads/rustediron1-alt2-bl/rustediron2_roughness.png",
    )
    .unwrap()
    .decode()
    .unwrap()
    .to_luma8();

    let mut out: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        image::ImageBuffer::new(metallic.width(), metallic.height());
    for x in 0..metallic.width() {
        for y in 0..metallic.height() {
            let m = metallic.get_pixel(x, y).0[0];
            let r = roughness.get_pixel(x, y).0[0];

            out.put_pixel(x, y, image::Rgba([0, r, m, 0]));
        }
    }

    let mr = Image::new(
        UVec2::new(out.width(), out.height()),
        TextureFormat::Rgba8UnormSrgb,
        out.into_raw(),
    );

    // for i in 0..10 {
    //     for j in 0..10 {
    //         commands.spawn(PbrBundle {
    //             mesh: meshes.insert(
    //                 shape::Box {
    //                     min_x: -0.5,
    //                     max_x: 0.5,
    //                     min_y: -0.5,
    //                     max_y: 0.5,
    //                     min_z: -0.5,
    //                     max_z: 0.5,
    //                 }
    //                 .into(),
    //             ),
    //             material: materials.insert(PbrMaterial {
    //                 base_color: Color([1.0, 1.0, 1.0, 1.0]),
    //                 // base_color_texture: Some(images.load("../assets/diffuse.png")),
    //                 base_color_texture: Some(images.load(
    //                     "/home/robert/Downloads/rustediron1-alt2-bl/rustediron2_basecolor.png",
    //                 )),
    //                 roughness: 1.0 / i as f32,
    //                 metallic: 1.0 / j as f32,
    //                 // normal_texture: Some(images.load("../assets/normal.png")),
    //                 normal_texture: Some(
    //                     images.load(
    //                         "/home/robert/Downloads/rustediron1-alt2-bl/rustediron2_normal.png",
    //                     ),
    //                 ),
    //                 metallic_roughness_texture: Some(images.insert(mr.clone())),
    //                 ..Default::default()
    //             }),
    //             transform: TransformBundle {
    //                 transform: Transform {
    //                     translation: Vec3::new(0.0 + i as f32, 1.0 + j as f32, -5.0),
    //                     // rotation: Quat::from_axis_angle(Vec3::Y, PI / 3.0),
    //                     ..Default::default()
    //                 },
    //                 ..Default::default()
    //             },
    //         });
    //     }
    // }

    // commands.spawn(PbrBundle {
    //     mesh: meshes.insert(
    //         shape::Box {
    //             min_x: -0.5,
    //             max_x: 0.5,
    //             min_y: -0.5,
    //             max_y: 0.5,
    //             min_z: -0.5,
    //             max_z: 0.5,
    //         }
    //         .into(),
    //     ),
    //     material: materials.insert(PbrMaterial {
    //         base_color: Color([1.0, 1.0, 1.0, 1.0]),
    //         base_color_texture: Some(images.load("../assets/diffuse.png")),
    //         // normal_texture: Some(images.load("../assets/normal.png")),
    //         ..Default::default()
    //     }),
    //     transform: TransformBundle {
    //         transform: Transform {
    //             translation: Vec3::new(0.0, 1.0, -5.0),
    //             // rotation: Quat::from_axis_angle(Vec3::Y, PI / 3.0),
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     },
    // });

    commands
        .spawn(PbrBundle {
            mesh: meshes.insert(
                shape::Box {
                    min_x: -0.1,
                    max_x: 0.1,
                    min_y: -0.1,
                    max_y: 0.1,
                    min_z: -0.1,
                    max_z: 0.1,
                }
                .into(),
            ),
            material: materials.insert(PbrMaterial {
                base_color: Color([1.0, 1.0, 1.0, 1.0]),
                ..Default::default()
            }),
            transform: TransformBundle {
                transform: Transform {
                    translation: Default::default(),
                    rotation: Quat::from_axis_angle(Vec3::Y, PI / 4.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        })
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
        commands.spawn(PbrBundle {
            mesh: meshes.insert(mesh.into()),
            material: materials.insert(PbrMaterial {
                base_color: Color(color),
                ..Default::default()
            }),
            transform: TransformBundle {
                transform: Transform::default(),
                ..Default::default()
            },
        });
    }
}

pub fn handle_selection(
    cursor: Res<Cursor>,
    mut windows: Query<&WindowState>,
    mut events: EventReader<MouseButtonInput>,
) {
    for event in events.iter() {
        let window = windows.get(cursor.window().unwrap()).unwrap();
        let size = window.inner_size();

        if event.state.is_pressed() && event.button.is_left() {
            let position = remap(
                cursor.position(),
                Vec2::new(size.width as f32, size.height as f32),
            );
        }
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
