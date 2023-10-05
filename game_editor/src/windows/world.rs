//! An immutable view of a scene.
mod node;
pub mod spawn_entity;

use std::sync::mpsc;

use bitflags::bitflags;
use game_common::components::transform::Transform;
use game_common::record::RecordReference;
use game_core::hierarchy::{Entity, TransformHierarchy};
use game_data::record::{Record, RecordBody};
use game_input::keyboard::KeyCode;
use game_input::mouse::{MouseButton, MouseMotion, MouseWheel};
use game_input::ButtonState;
use game_render::camera::{Camera, RenderTarget};
use game_render::color::Color;
use game_render::entities::CameraId;
use game_render::light::{DirectionalLight, PointLight, SpotLight};
use game_render::{shape, Renderer};
use game_scene::scene::{Material, Node, Scene};
use game_scene::Scenes;
use game_ui::reactive::{ReadSignal, Scope, WriteSignal};
use game_ui::style::{
    Background, BorderRadius, Bounds, Direction, Growth, Justify, Size, SizeVec2, Style,
};
use game_ui::widgets::{Button, Container, ParseInput, Text};
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use glam::{Quat, Vec2, Vec3};
use parking_lot::Mutex;

use crate::state::EditorState;
use crate::world::selection;

use super::SpawnWindow;

const ZOOM_DISTANCE_MIN: f32 = 0.2;
const ZOOM_DISTANCE_MAX: f32 = 100.0;
const ZOOM_FACTOR: f32 = 0.15 / 120.0;

pub struct WorldWindowState {
    camera: CameraId,
    camera_controller: CameraController,
    // TODO: Use `Cursor` instead of adding our own thing.
    cursor: Vec2,
    edit_mode: EditMode,
    edit_op: Option<EditOperation>,
    state: State,
}

impl WorldWindowState {
    pub fn new(
        state: State,
        renderer: &mut Renderer,
        window_id: WindowId,
        scenes: &mut Scenes,
        hierarchy: &mut TransformHierarchy,
    ) -> Self {
        let camera = renderer.entities.cameras.insert(Camera {
            projection: Default::default(),
            target: RenderTarget::Window(window_id),
            transform: Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
        });

        renderer.entities.point_lights.insert(PointLight {
            transform: Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
            intensity: 70.0,
            radius: 100.0,
            color: Color::WHITE,
        });

        let plane = hierarchy.append(None, Transform::default());

        scenes.insert(
            plane,
            Scene {
                nodes: vec![Node {
                    transform: Transform::default(),
                    mesh: 0,
                    material: 0,
                }],
                meshes: vec![shape::Plane { size: 100.0 }.into()],
                materials: vec![Material::default()],
                images: vec![],
            },
        );

        state.entities.update(|e| e.push(plane));

        let s = hierarchy.append(None, Transform::default());
        scenes.load(s, "../../sponza.glb");

        Self {
            camera,
            camera_controller: CameraController::default(),
            cursor: Vec2::ZERO,
            edit_mode: EditMode::None,
            edit_op: None,
            state,
        }
    }

    pub fn handle_event(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        event: WindowEvent,
        window: WindowId,
        hierarchy: &mut TransformHierarchy,
    ) {
        let mut camera = renderer.entities.cameras.get_mut(self.camera).unwrap();

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
            WindowEvent::CursorMoved(event) => {
                self.cursor = event.position;

                let c = camera.clone();
                drop(camera);
                self.update_edit_op(renderer, scenes, window, c, hierarchy);
            }
            WindowEvent::KeyboardInput(event) => {
                if event.key_code == Some(KeyCode::LShift) {
                    match event.state {
                        ButtonState::Pressed => self.camera_controller.mode |= Mode::SHIFT,
                        ButtonState::Released => self.camera_controller.mode &= !Mode::SHIFT,
                    }
                }

                match event.key_code {
                    // Front view
                    Some(KeyCode::Numpad1) => {
                        let distance =
                            (self.camera_controller.origin - camera.transform.translation).length();

                        camera.transform.translation =
                            self.camera_controller.origin + Vec3::new(0.0, 0.0, distance);
                        camera.transform = camera.transform.looking_to(-Vec3::Z, Vec3::Y);
                    }
                    // Right view
                    Some(KeyCode::Numpad3) => {
                        let distance =
                            (self.camera_controller.origin - camera.transform.translation).length();

                        camera.transform.translation =
                            self.camera_controller.origin + Vec3::new(distance, 0.0, 0.0);
                        camera.transform = camera.transform.looking_to(-Vec3::X, Vec3::Y);
                    }
                    // Top view
                    Some(KeyCode::Numpad7) => {
                        let distance = (self.camera_controller.origin
                            - camera.transform.translation)
                            .length()
                            .abs();

                        camera.transform.translation =
                            self.camera_controller.origin + Vec3::new(0.0, distance, 0.0);
                        camera.transform = camera.transform.looking_to(-Vec3::Y, Vec3::Z);
                    }
                    _ => (),
                }

                let camera = {
                    let c = camera.clone();
                    drop(camera);
                    c
                };
                if event.state.is_pressed() && !self.state.selection.with(|v| v.is_empty()) {
                    match event.key_code {
                        Some(KeyCode::Escape) => {
                            self.reset_edit_op(renderer, scenes, hierarchy);
                            self.edit_mode = EditMode::None;
                        }
                        Some(KeyCode::G) => {
                            self.edit_mode = EditMode::Translate(None);
                            self.create_edit_op(renderer, scenes, hierarchy);
                        }
                        Some(KeyCode::R) => {
                            self.edit_mode = EditMode::Rotate(None);
                            self.create_edit_op(renderer, scenes, hierarchy);
                        }
                        Some(KeyCode::S) => {
                            self.edit_mode = EditMode::Scale(None);
                            self.create_edit_op(renderer, scenes, hierarchy);
                        }
                        Some(KeyCode::X) => {
                            match &mut self.edit_mode {
                                EditMode::Translate(axis) => *axis = Some(Axis::X),
                                EditMode::Rotate(axis) => *axis = Some(Axis::X),
                                EditMode::Scale(axis) => *axis = Some(Axis::X),
                                EditMode::None => (),
                            }

                            if self.edit_mode != EditMode::None {
                                let camera = camera.clone();
                                self.reset_edit_op(renderer, scenes, hierarchy);
                                self.update_edit_op(renderer, scenes, window, camera, hierarchy);
                            }
                        }
                        Some(KeyCode::Y) => {
                            match &mut self.edit_mode {
                                EditMode::Translate(axis) => *axis = Some(Axis::Y),
                                EditMode::Rotate(axis) => *axis = Some(Axis::Y),
                                EditMode::Scale(axis) => *axis = Some(Axis::Y),
                                EditMode::None => (),
                            }

                            if self.edit_mode != EditMode::None {
                                let camera = camera.clone();
                                self.reset_edit_op(renderer, scenes, hierarchy);
                                self.update_edit_op(renderer, scenes, window, camera, hierarchy);
                            }
                        }
                        Some(KeyCode::Z) => {
                            match &mut self.edit_mode {
                                EditMode::Translate(axis) => *axis = Some(Axis::Z),
                                EditMode::Rotate(axis) => *axis = Some(Axis::Z),
                                EditMode::Scale(axis) => *axis = Some(Axis::Z),
                                EditMode::None => (),
                            };

                            if self.edit_mode != EditMode::None {
                                let camera = camera.clone();
                                self.reset_edit_op(renderer, scenes, hierarchy);
                                self.update_edit_op(renderer, scenes, window, camera, hierarchy);
                            }
                        }
                        _ => (),
                    }
                }
            }
            WindowEvent::MouseButtonInput(event) => match event.button {
                MouseButton::Left => {
                    if !event.state.is_pressed() {
                        return;
                    }

                    drop(camera);
                    if self.edit_mode == EditMode::None {
                        self.update_selection(renderer, scenes, window);
                    } else {
                        self.confirm_edit_op(renderer);
                    }
                }
                MouseButton::Right => {
                    if self.edit_mode != EditMode::None {
                        drop(camera);
                        self.reset_edit_op(renderer, scenes, hierarchy);
                        self.edit_mode = EditMode::None;
                    }
                }
                MouseButton::Middle => match event.state {
                    ButtonState::Pressed => self.camera_controller.mode |= Mode::MIDDLE,
                    ButtonState::Released => self.camera_controller.mode &= !Mode::MIDDLE,
                },
                _ => (),
            },
            _ => todo!(),
        }
    }

    fn update_selection(&mut self, renderer: &mut Renderer, scenes: &mut Scenes, id: WindowId) {
        let camera = renderer
            .entities
            .cameras
            .get_mut(self.camera)
            .unwrap()
            .clone();
        let viewport_size = renderer.get_surface_size(id).unwrap().as_vec2();

        self.state.selection.update(|v| v.clear());
        for id in &self.state.entities.get() {
            let object = scenes.objects(*id).unwrap().nth(0).unwrap();
            let object = renderer.entities.objects.get(object).unwrap();

            let mesh = renderer.meshes.get(object.mesh).unwrap();

            if let Some(aabb) = mesh.compute_aabb() {
                let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

                if selection::hit_test(ray, aabb) {
                    self.state.selection.update(|v| v.push(*id));
                }
            }
        }
    }

    fn create_edit_op(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        hierarchy: &mut TransformHierarchy,
    ) {
        let mut entities = Vec::new();

        for id in &self.state.selection.get() {
            let transform = hierarchy.get(*id).unwrap();

            entities.push(EditEntity {
                id: *id,
                origin: transform,
            });
        }

        self.edit_op = Some(EditOperation {
            cursor_origin: self.cursor,
            entities,
        });
    }

    fn update_edit_op(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        window: WindowId,
        camera: Camera,
        hierarchy: &mut TransformHierarchy,
    ) {
        let viewport_size = renderer.get_surface_size(window).unwrap().as_vec2();

        let camera_rotation = camera.transform.rotation;
        let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

        match self.edit_mode {
            EditMode::Translate(axis) => {
                for id in &self.state.selection.get() {
                    let transform = hierarchy.get_mut(*id).unwrap();

                    // Find the intersection of the camera ray with the plane placed
                    // at the object, facing the camera. The projected point is the new
                    // translation.
                    let plane_origin = transform.translation;
                    let plane_normal = camera_rotation * Vec3::Z;
                    // FIXME: What if no intersection?
                    let point = ray.plane_intersection(plane_origin, plane_normal).unwrap();

                    match axis {
                        Some(Axis::X) => transform.translation.x = point.x,
                        Some(Axis::Y) => transform.translation.y = point.y,
                        Some(Axis::Z) => transform.translation.z = point.z,
                        None => transform.translation = point,
                    }

                    self.state
                        .transform
                        .set_translation
                        .update(|translation| *translation = transform.translation);
                }
            }
            EditMode::None => (),
            _ => todo!(),
        }
    }

    fn reset_edit_op(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        hierarchy: &mut TransformHierarchy,
    ) {
        let Some(op) = &self.edit_op else {
            return;
        };

        for entity in &op.entities {
            *hierarchy.get_mut(entity.id).unwrap() = entity.origin;
        }
    }

    fn confirm_edit_op(&mut self, renderer: &mut Renderer) {
        self.edit_op = None;
        self.edit_mode = EditMode::None;
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        hierarchy: &mut TransformHierarchy,
    ) {
        while let Ok(event) = self.state.rx.try_recv() {
            match event {
                Event::Spawn(record_ref) => {
                    // It is possible the record is already deleted once we
                    // receive this event.
                    if let Some(record) = self
                        .state
                        .state
                        .records
                        .get(record_ref.module, record_ref.record)
                    {
                        self.spawn_entity(renderer, scenes, record, hierarchy);
                    }
                }
                Event::SpawnDirectionalLight => {
                    renderer
                        .entities
                        .directional_lights
                        .insert(DirectionalLight {
                            transform: Transform::default(),
                            color: Color::WHITE,
                            illuminance: 100_000.0,
                        });
                }
                Event::SpawnPointLight => {
                    renderer.entities.point_lights.insert(PointLight {
                        transform: Transform::default(),
                        color: Color::WHITE,
                        intensity: 70.0,
                        radius: 50.0,
                    });
                }
                Event::SpawnSpotLight => {
                    renderer.entities.spot_lights.insert(SpotLight {
                        transform: Transform::default(),
                        color: Color::WHITE,
                        intensity: 70.0,
                        radius: 50.0,
                        inner_cutoff: 45.0,
                        outer_cutoff: 50.0,
                    });
                }
            }
        }
    }

    fn spawn_entity(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        record: Record,
        hierarchy: &mut TransformHierarchy,
    ) {
        match record.body {
            RecordBody::Object(object) => {
                let entity = hierarchy.append(None, Transform::default());
                let model = object.uri.as_ref().to_path_buf();
                scenes.load(entity, model);

                self.state.entities.update(|entities| entities.push(entity));
            }
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug)]
struct EditOperation {
    cursor_origin: Vec2,
    entities: Vec<EditEntity>,
}

#[derive(Clone, Debug)]
struct EditEntity {
    id: Entity,
    origin: Transform,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
enum EditMode {
    #[default]
    None,
    Translate(Option<Axis>),
    Rotate(Option<Axis>),
    Scale(Option<Axis>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Axis {
    X,
    Y,
    Z,
}

pub struct State {
    rx: mpsc::Receiver<Event>,
    entities: WriteSignal<Vec<Entity>>,
    selection: WriteSignal<Vec<Entity>>,
    transform: TransformState,
    state: EditorState,
}

pub fn build_ui(cx: &Scope, state: EditorState) -> State {
    // let cx = cx.append(Area::new());

    let (tx, rx) = mpsc::channel();

    let style = Style {
        background: Background::GRAY,
        growth: Growth::splat(1.0),
        bounds: Bounds::exact(SizeVec2 {
            x: Size::Pixels(300),
            y: Size::Pixels(2000), // y: Size::INFINITY,
        }),
        ..Default::default()
    };

    let root = cx.append(Container::new().style(style));

    let header = root.append(Container::new().style(Style {
        direction: Direction::Column,
        ..Default::default()
    }));

    let writer = tx.clone();
    let state2 = state.clone();
    let new_button = header.append(Button::new().on_click(move |_| {
        let writer = writer.clone();
        state2.spawn_windows.send(SpawnWindow::SpawnEntity(writer));
    }));
    new_button.append(Text::new().text("Spawn new"));

    let (entities, set_entities) = cx.create_signal(Vec::new());
    let (selection, set_selection) = cx.create_signal(Vec::new());

    let nodes = Mutex::new(vec![]);
    let cx2 = root.clone();
    {
        let set_selection = set_selection.clone();
        root.create_effect(move || {
            let entities = entities.get();
            let selection = selection.get();

            let mut nodes = nodes.lock();
            for id in nodes.drain(..) {
                cx2.remove(id);
            }

            for entity in entities {
                let is_selected = selection.contains(&entity);

                let style = Style {
                    background: if is_selected {
                        Background::YELLOW
                    } else {
                        Background::None
                    },
                    ..Default::default()
                };

                let on_click = {
                    let set_selection = set_selection.clone();
                    move |_| {
                        set_selection.update(|v| {
                            v.clear();
                            v.push(entity);
                        });
                    }
                };

                let button = cx2.append(Button::new().style(style).on_click(on_click));
                button.append(Text::new().text(format!("{:?}", entity)));

                nodes.push(button.id().unwrap());
            }
        });
    }

    let transform = build_object_transform(&root);

    State {
        entities: set_entities,
        selection: set_selection,
        transform,
        rx,
        state,
    }
}

struct TransformState {
    translation: ReadSignal<Vec3>,
    rotation: ReadSignal<Quat>,
    set_translation: WriteSignal<Vec3>,
    set_rotation: WriteSignal<Quat>,
}

fn build_object_transform(cx: &Scope) -> TransformState {
    let root = cx.append(Container::new());

    let (translation, set_translation) = root.create_signal(Vec3::ZERO);
    let (rotation, set_rotation) = root.create_signal(Quat::IDENTITY);

    {
        let translation_row = cx.append(Container::new().style(Style {
            growth: Growth::x(1.0),
            direction: Direction::Column,
            justify: Justify::SpaceBetween,
            border_radius: BorderRadius::splat(Size::Pixels(5)),
            ..Default::default()
        }));

        let style = Style {
            ..Default::default()
        };

        let set_x = {
            let set_translation = set_translation.clone();
            move |val| {
                set_translation.update(|translation| translation.x = val);
            }
        };

        let set_y = {
            let set_translation = set_translation.clone();
            move |val| {
                set_translation.update(|translation| translation.y = val);
            }
        };

        let set_z = {
            let set_translation = set_translation.clone();
            move |val| {
                set_translation.update(|translation| translation.z = val);
            }
        };

        let wrapper_style = Style {
            growth: Growth::x(1.0),
            direction: Direction::Column,
            ..Default::default()
        };

        let x = translation_row.append(Container::new().style(wrapper_style.clone()));
        x.append(Text::new().text("X"));
        x.append(ParseInput::new(0.0).style(style.clone()).on_change(set_x));

        let y = translation_row.append(Container::new().style(wrapper_style.clone()));
        y.append(Text::new().text("Y"));
        y.append(ParseInput::new(0.0).style(style.clone()).on_change(set_y));

        let z = translation_row.append(Container::new().style(wrapper_style));
        z.append(Text::new().text("Z"));
        z.append(ParseInput::new(0.0).style(style).on_change(set_z));
    }

    {
        let row = cx.append(Container::new().style(Style {
            growth: Growth::x(1.0),
            direction: Direction::Column,
            justify: Justify::SpaceBetween,
            border_radius: BorderRadius::splat(Size::Pixels(5)),
            ..Default::default()
        }));

        let wrapper_style = Style {
            growth: Growth::x(1.0),
            direction: Direction::Column,
            ..Default::default()
        };

        let set_x = {
            let set_rotation = set_rotation.clone();
            move |val| {
                set_rotation.update(|rotation| rotation.x = val);
            }
        };

        let set_y = {
            let set_rotation = set_rotation.clone();
            move |val| {
                set_rotation.update(|rotation| rotation.y = val);
            }
        };

        let set_z = {
            let set_rotation = set_rotation.clone();
            move |val| {
                set_rotation.update(|rotation| rotation.z = val);
            }
        };

        let set_w = {
            let set_rotation = set_rotation.clone();
            move |val| {
                set_rotation.update(|rotation| rotation.w = val);
            }
        };

        let x = row.append(Container::new().style(wrapper_style.clone()));
        x.append(Text::new().text("X"));
        x.append(ParseInput::new(0.0).on_change(set_x));

        let y = row.append(Container::new().style(wrapper_style.clone()));
        y.append(Text::new().text("Y"));
        y.append(ParseInput::new(0.0).on_change(set_y));

        let z = row.append(Container::new().style(wrapper_style.clone()));
        z.append(Text::new().text("Z"));
        z.append(ParseInput::new(0.0).on_change(set_z));

        let w = row.append(Container::new().style(wrapper_style));
        w.append(Text::new().text("W"));
        w.append(ParseInput::new(1.0).on_change(set_w));
    }

    TransformState {
        translation,
        rotation,
        set_translation,
        set_rotation,
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Event {
    Spawn(RecordReference),
    SpawnDirectionalLight,
    SpawnPointLight,
    SpawnSpotLight,
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
        let distance = (camera_transform.translation - self.origin).length();
        let factor = ZOOM_FACTOR * event.y;
        let offset = f32::clamp(
            distance * (1.0 + factor),
            ZOOM_DISTANCE_MIN,
            ZOOM_DISTANCE_MAX,
        );

        let dir = camera_transform.rotation * -Vec3::Z;
        camera_transform.translation = self.origin - (dir * offset);
    }

    fn update(&mut self, camera_transform: &mut Transform, event: MouseMotion) {
        match self.mode {
            mode if mode == Mode::TRANSLATE => {
                let x = -event.delta.x * 0.01;
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
