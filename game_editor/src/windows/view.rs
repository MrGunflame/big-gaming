//! An immutable view of a scene.

use bitflags::bitflags;
use game_common::components::transform::Transform;
use game_input::mouse::{MouseButton, MouseMotion, MouseWheel};
use game_input::ButtonState;
use game_render::camera::{Camera, RenderTarget};
use game_render::color::Color;
use game_render::entities::{CameraId, Object};
use game_render::pbr::PbrMaterial;
use game_render::{shape, RenderState};
use game_window::events::{VirtualKeyCode, WindowEvent};
use game_window::windows::WindowId;
use glam::{Quat, Vec3};

pub struct WorldWindowState {
    camera: CameraId,
    camera_controller: CameraController,
}

impl WorldWindowState {
    pub fn new(renderer: &mut RenderState, window_id: WindowId) -> Self {
        let camera = renderer.entities.cameras().insert(Camera {
            projection: Default::default(),
            target: RenderTarget::Window(window_id),
            transform: Transform::default(),
        });

        renderer.entities.objects().insert(Object {
            transform: Transform::default(),
            material: renderer.materials.insert(PbrMaterial {
                base_color: Color::WHITE,
                ..Default::default()
            }),
            mesh: renderer.meshes.insert(shape::Plane { size: 100.0 }.into()),
        });

        Self {
            camera,
            camera_controller: CameraController::default(),
        }
    }

    pub fn handle_event(&mut self, renderer: &mut RenderState, event: WindowEvent) {
        let camera = renderer.entities.cameras().get_mut(self.camera).unwrap();

        match event {
            WindowEvent::MouseMotion(event) => {
                self.camera_controller.update(&mut camera.transform, event);
            }
            WindowEvent::MouseWheel(event) => {
                self.camera_controller.zoom(&mut camera.transform, event);
            }
            WindowEvent::CursorLeft(_) => {
                // Reset the mode when the cursor leaves the window.
                self.camera_controller.mode = Mode::NONE;
            }
            WindowEvent::KeyboardInput(event) => {
                if event.key_code == Some(VirtualKeyCode::LShift) {
                    match event.state {
                        ButtonState::Pressed => self.camera_controller.mode |= Mode::SHIFT,
                        ButtonState::Released => self.camera_controller.mode &= !Mode::SHIFT,
                    }
                }
            }
            WindowEvent::MouseButtonInput(event) => {
                if event.button == MouseButton::Middle {
                    match event.state {
                        ButtonState::Pressed => self.camera_controller.mode |= Mode::MIDDLE,
                        ButtonState::Released => self.camera_controller.mode &= !Mode::MIDDLE,
                    }
                }
            }
            _ => todo!(),
        }
    }
}

// let id = commands
//     .spawn(Window {
//         title: "test".to_owned(),
//     })
//     .insert(ViewWindowState::default())
//     .id();

// commands
//     .spawn(CameraBundle {
//         camera: Camera {
//             projection: Default::default(),
//             target: RenderTarget::Window(id),
//         },
//         transform: Transform::default(),
//     })
//     .insert(ViewCamera);

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

// commands.spawn(PbrBundle {
//     mesh: meshes.insert(shape::Plane { size: 100.0 }.into()),
//     material: materials.insert(PbrMaterial {
//         base_color: Color([1.0, 1.0, 1.0, 1.0]),
//         ..Default::default()
//     }),
//     transform: TransformBundle {
//         transform: Transform {
//             translation: Vec3::new(0.0, -1.0, 0.0),
//             ..Default::default()
//         },
//         ..Default::default()
//     },
// });

// commands.spawn(DirectionalLightBundle {
//     light: DirectionalLight {
//         color: Color::WHITE,
//         illuminance: 1.0,
//     },
//     transform: TransformBundle {
//         transform: Transform {
//             translation: Vec3::new(1.0, 0.0, 1.0),
//             ..Default::default()
//         },
//         ..Default::default()
//     },
// });

// commands.spawn(PointLightBundle {
//     light: PointLight {
//         color: Color::WHITE,
//         intensity: 60.0,
//         radius: 100.0,
//     },
//     transform: TransformBundle {
//         transform: Transform {
//             translation: Vec3::new(0.0, 2.0, 0.0),
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
//     scene: scenes.load("/home/robert/projects/game/client/tools/gltf2model/sponza.model"),
//     transform: TransformBundle {
//         transform: Transform {
//             translation: Vec3::new(0.0, 0.0, 0.0),
//             ..Default::default()
//         },
//         ..Default::default()
//     },
// });

// let metallic = image::io::Reader::open(
//     "/home/robert/Downloads/rustediron1-alt2-bl/rustediron2_metallic.png",
// )
// .unwrap()
// .decode()
// .unwrap()
// .to_luma8();

// let roughness = image::io::Reader::open(
//     "/home/robert/Downloads/rustediron1-alt2-bl/rustediron2_roughness.png",
// )
// .unwrap()
// .decode()
// .unwrap()
// .to_luma8();

// let mut out: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
//     image::ImageBuffer::new(metallic.width(), metallic.height());
// for x in 0..metallic.width() {
//     for y in 0..metallic.height() {
//         let m = metallic.get_pixel(x, y).0[0];
//         let r = roughness.get_pixel(x, y).0[0];

//         out.put_pixel(x, y, image::Rgba([0, r, m, 0]));
//     }
// }

// let mr = Image::new(
//     UVec2::new(out.width(), out.height()),
//     TextureFormat::Rgba8UnormSrgb,
//     out.into_raw(),
// );

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

// commands
//     .spawn(PbrBundle {
//         mesh: meshes.insert(
//             shape::Box {
//                 min_x: -0.1,
//                 max_x: 0.1,
//                 min_y: -0.1,
//                 max_y: 0.1,
//                 min_z: -0.1,
//                 max_z: 0.1,
//             }
//             .into(),
//         ),
//         material: materials.insert(PbrMaterial {
//             base_color: Color([1.0, 1.0, 1.0, 1.0]),
//             ..Default::default()
//         }),
//         transform: TransformBundle {
//             transform: Transform {
//                 translation: Default::default(),
//                 rotation: Quat::from_axis_angle(Vec3::Y, PI / 4.0),
//                 ..Default::default()
//             },
//             ..Default::default()
//         },
//     })
//     .insert(OriginMarker);

// for (mesh, color) in [
//     (
//         shape::Box {
//             min_x: 0.0,
//             max_x: 1.0,
//             min_y: -0.1,
//             max_y: 0.1,
//             min_z: -0.1,
//             max_z: 0.1,
//         },
//         [1.0, 0.0, 0.0, 1.0],
//     ),
//     (
//         shape::Box {
//             min_x: -0.1,
//             max_x: 0.1,
//             min_y: 0.0,
//             max_y: 1.0,
//             min_z: -0.1,
//             max_z: 0.1,
//         },
//         [0.0, 1.0, 0.0, 1.0],
//     ),
//     (
//         shape::Box {
//             min_x: -0.1,
//             max_x: 0.1,
//             min_y: -0.1,
//             max_y: 0.1,
//             min_z: 0.0,
//             max_z: 1.0,
//         },
//         [0.0, 0.0, 1.0, 1.0],
//     ),
// ] {
//     commands.spawn(PbrBundle {
//         mesh: meshes.insert(mesh.into()),
//         material: materials.insert(PbrMaterial {
//             base_color: Color(color),
//             ..Default::default()
//         }),
//         transform: TransformBundle {
//             transform: Transform::default(),
//             ..Default::default()
//         },
//     });
// }

#[derive(Clone, Debug, Default)]
struct CameraController {
    origin: Vec3,
    mode: Mode,
}

impl CameraController {
    fn zoom(&self, camera_transform: &mut Transform, event: MouseWheel) {
        let dir = camera_transform.rotation * -Vec3::Z;
        camera_transform.translation -= dir * event.y * 0.05;
    }

    fn update(&mut self, camera_transform: &mut Transform, event: MouseMotion) {
        match self.mode {
            mode if mode == Mode::TRANSLATE => {
                let x = event.delta.x * 0.01;
                let y = event.delta.y * 0.01;

                let mut distance = (camera_transform.rotation * Vec3::X) * x;
                distance += (camera_transform.rotation * Vec3::Y) * y;

                camera_transform.translation += distance;
                self.origin += distance;
            }
            mode if mode == Mode::ROTATE => {
                let x = event.delta.x * 0.01;
                let y = event.delta.y * 0.01;

                // // Rotate around origin with a constant distance.
                let distance = (camera_transform.translation - self.origin).length().abs();

                let q1 = Quat::from_axis_angle(Vec3::Y, -x);
                let q2 = Quat::from_axis_angle(Vec3::X, -y);

                camera_transform.rotation = q1 * camera_transform.rotation;
                camera_transform.rotation = camera_transform.rotation * q2;

                // Renormalize quat due to FP error creep.
                if camera_transform.rotation.is_normalized() {
                    camera_transform.rotation = camera_transform.rotation.normalize();
                }

                // FIXME: FP error creep means that distance will very slowly grow
                // over time. Storing the radius instead of computing the distance
                // would fix this.
                camera_transform.translation =
                    self.origin + camera_transform.rotation * Vec3::new(0.0, 0.0, distance);
            }
            _ => (),
        }
    }
}

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
