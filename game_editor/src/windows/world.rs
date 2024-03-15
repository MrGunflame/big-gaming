//! An immutable view of a scene.
mod edit;
mod hierarchy;
mod node;
mod panel;
pub mod spawn_entity;

use std::sync::mpsc;

use bitflags::bitflags;
use game_common::components::{Color, PointLight, PrimaryCamera};
use game_common::components::{MeshInstance, Transform};
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::World;
use game_core::hierarchy::Key;
use game_input::keyboard::KeyCode;
use game_input::mouse::{MouseButton, MouseMotion, MouseWheel};
use game_input::ButtonState;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::Renderer;
use game_ui::reactive::{ReadSignal, Scope, WriteSignal};
use game_ui::style::{BorderRadius, Direction, Growth, Justify, Size, Style};
use game_ui::widgets::{Container, ParseInput, Text, ValueProvider};
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use glam::{Quat, Vec2, Vec3};

use crate::state::EditorState;

use self::edit::{EditMode, EditOperation};
use self::panel::{Entity, Panel};

const ZOOM_DISTANCE_MIN: f32 = 0.2;
const ZOOM_DISTANCE_MAX: f32 = 100.0;
const ZOOM_FACTOR: f32 = 0.15 / 120.0;

pub struct WorldWindowState {
    camera: EntityId,
    camera_controller: CameraController,
    // TODO: Use `Cursor` instead of adding our own thing.
    cursor: Vec2,
    state: State,
    edit_op: EditOperation,
}

impl WorldWindowState {
    pub fn new(state: State, window_id: WindowId, world: &mut World) -> Self {
        let camera = world.spawn();
        world.insert_typed(
            camera,
            Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
        );
        world.insert_typed(camera, PrimaryCamera);

        let mut light = world.spawn();
        world.insert_typed(light, Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)));
        world.insert_typed(
            light,
            PointLight {
                color: Color::WHITE,
                intensity: 100.0,
                radius: 100.0,
            },
        );

        let mut obj = world.spawn();
        world.insert_typed(obj, Transform::default());
        world.insert_typed(
            obj,
            MeshInstance {
                path: "../game_client/sponza.glb".into(),
            },
        );

        state.entities.set(vec![
            Entity {
                id: light,
                name: "Point Light".into(),
                is_selected: false,
            },
            Entity {
                id: obj,
                name: "Obj".into(),
                is_selected: false,
            },
        ]);

        Self {
            camera,
            camera_controller: CameraController::default(),
            cursor: Vec2::ZERO,
            edit_op: EditOperation::new(),
            state,
        }
    }

    pub fn handle_event(
        &mut self,
        world: &mut World,
        event: WindowEvent,
        window: WindowId,
        renderer: &mut Renderer,
    ) {
        let viewport_size = renderer.get_surface_size(window).unwrap();

        let mut camera_transform = world.get_typed::<Transform>(self.camera);
        let mut camera = Camera {
            transform: camera_transform,
            projection: Projection::default(),
            target: RenderTarget::Window(window),
        };
        camera.update_aspect_ratio(viewport_size);

        match event {
            WindowEvent::MouseMotion(event) => {
                self.camera_controller.update(&mut camera_transform, event);
            }
            WindowEvent::MouseWheel(event) => {
                self.camera_controller.zoom(&mut camera_transform, event);
            }
            WindowEvent::CursorLeft(_) => {
                // Reset the mode when the cursor leaves the window.
                self.camera_controller.mode = Mode::NONE;
            }
            WindowEvent::CursorMoved(event) => {
                self.cursor = event.position;

                self.update_edit_op(world, camera, viewport_size.as_vec2());
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
                    Some(KeyCode::Delete) => {
                        // let keys: Vec<_> = self
                        //     .state
                        //     .selection
                        //     .update(|selection| selection.drain().collect());

                        // self.state.nodes.update(|nodes| {
                        //     for key in keys {
                        //         nodes.remove(key);
                        //         self.state.events.push_back(Event::Destroy { node: key });
                        //     }
                        // });
                    }
                    _ => (),
                }

                if event.state.is_pressed()
                /*&& !self.state.selection.with(|v| v.is_empty())*/
                {
                    match event.key_code {
                        Some(KeyCode::Escape) => {
                            self.reset_edit_op(world);
                            self.edit_op.set_mode(EditMode::None);
                        }
                        Some(KeyCode::G) => {
                            self.edit_op.set_mode(EditMode::Translate(None));
                            // self.create_edit_op(world, camera, viewport_size.as_vec2());
                        }
                        Some(KeyCode::R) => {
                            self.edit_op.set_mode(EditMode::Rotate(None));
                            // self.create_edit_op(world, camera, viewport_size.as_vec2());
                        }
                        Some(KeyCode::S) => {
                            self.edit_op.set_mode(EditMode::Scale(None));
                            // self.create_edit_op(world, camera, viewport_size.as_vec2());
                        }
                        Some(KeyCode::X) => {
                            let mode = match self.edit_op.mode() {
                                EditMode::Translate(_) => EditMode::Translate(Some(Axis::X)),
                                EditMode::Rotate(_) => EditMode::Rotate(Some(Axis::X)),
                                EditMode::Scale(_) => EditMode::Scale(Some(Axis::X)),
                                EditMode::None => EditMode::None,
                            };
                            self.edit_op.set_mode(mode);

                            if self.edit_op.mode() != EditMode::None {
                                let camera = camera.clone();
                                self.update_edit_op(world, camera, viewport_size.as_vec2());
                            }
                        }
                        Some(KeyCode::Y) => {
                            let mode = match self.edit_op.mode() {
                                EditMode::Translate(_) => EditMode::Translate(Some(Axis::Y)),
                                EditMode::Rotate(_) => EditMode::Rotate(Some(Axis::Y)),
                                EditMode::Scale(_) => EditMode::Scale(Some(Axis::Y)),
                                EditMode::None => EditMode::None,
                            };
                            self.edit_op.set_mode(mode);

                            if self.edit_op.mode() != EditMode::None {
                                let camera = camera.clone();
                                self.update_edit_op(world, camera, viewport_size.as_vec2());
                            }
                        }
                        Some(KeyCode::Z) => {
                            let mode = match self.edit_op.mode() {
                                EditMode::Translate(_) => EditMode::Translate(Some(Axis::Z)),
                                EditMode::Rotate(_) => EditMode::Rotate(Some(Axis::Z)),
                                EditMode::Scale(_) => EditMode::Scale(Some(Axis::Z)),
                                EditMode::None => EditMode::None,
                            };
                            self.edit_op.set_mode(mode);

                            if self.edit_op.mode() != EditMode::None {
                                let camera = camera.clone();
                                self.update_edit_op(world, camera, viewport_size.as_vec2());
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

                    if self.edit_op.mode() == EditMode::None {
                        // self.update_selection(renderer, scenes, window);
                    } else {
                        self.confirm_edit_op(renderer);
                    }
                }
                MouseButton::Right => {
                    if self.edit_op.mode() != EditMode::None {
                        self.reset_edit_op(world);
                        self.edit_op.set_mode(EditMode::None);
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

        world.insert_typed(self.camera, camera_transform);
    }

    // fn update_selection(&mut self, renderer: &mut Renderer, scenes: &mut SceneState, id: WindowId) {
    //     let camera = renderer
    //         .entities
    //         .cameras
    //         .get_mut(self.camera)
    //         .unwrap()
    //         .clone();
    //     let viewport_size = renderer.get_surface_size(id).unwrap().as_vec2();

    //     for (node, entity) in self.node_map.iter() {
    //         let Some(object) = scenes.entities.mesh_instance(*entity) else {
    //             continue;
    //         };
    //         let object = renderer.entities.objects.get(object).unwrap();

    //         let mesh = renderer.meshes.get(object.mesh).unwrap();

    //         if let Some(aabb) = mesh.compute_aabb() {
    //             let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

    //             if selection::hit_test(ray, aabb) {
    //                 self.state.selection.update(|v| v.insert(*node));
    //             }
    //         }
    //     }
    // }

    // fn create_edit_op(&mut self, world: &mut World, camera: Camera, viewport_size: Vec2) {
    //     let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

    //     self.edit_op.create(self.cursor, ray);

    //     self.state.selection.with(|selection| {
    //         for id in selection {
    //             let transform = self
    //                 .state
    //                 .nodes
    //                 .with(|nodes| nodes.get(*id).unwrap().transform);

    //             self.edit_op.push(*id, transform);
    //         }
    //     });
    // }

    fn update_edit_op(&mut self, world: &mut World, camera: Camera, viewport_size: Vec2) {
        let camera_rotation = camera.transform.rotation;
        let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

        for (entity, transform) in self.edit_op.update(ray, camera_rotation) {
            world.insert_typed(entity, transform);

            // self.state.props.update(|props| props.transform = transform);
        }
    }

    fn reset_edit_op(&mut self, world: &mut World) {
        for (entity, transform) in self.edit_op.reset() {
            world.insert_typed(entity, transform);

            // self.state.props.update(|props| props.transform = transform);
        }
    }

    fn confirm_edit_op(&mut self, renderer: &mut Renderer) {
        self.edit_op.set_mode(EditMode::None);
        self.edit_op.confirm();
    }

    pub fn update(&mut self, world: &mut World) {
        // while let Ok(event) = self.state.rx.try_recv() {
        //     self.state.events.push_back(event);
    }

    // while let Some(event) = self.state.events.pop_front() {
    //     match event {
    //         Event::UpdateSelection { node, additive } => {
    //             // self.state.selection.update(|selection| {
    //             //     if !additive {
    //             //         selection.clear();
    //             //     }

    //             //     selection.insert(node);
    //             // });

    //             // FIXME: We select the most-recent node right now. Need to
    //             // figure out what to display when selecting multiple nodes.
    //             // let transform = self
    //             //     .state
    //             //     .nodes
    //             //     .with(|hierarchy| hierarchy.get(node).unwrap().transform);
    //             // self.state.props.update(|props| {
    //             //     props.transform = transform;
    //             // });
    //         }
    //         Event::Spawn(record_ref) => {
    //             // It is possible the record is already deleted once we
    //             // receive this event.
    //             // if let Some(record) = self
    //             //     .state
    //             //     .state
    //             //     .records
    //             //     .get(record_ref.module, record_ref.record)
    //             // {
    //             //     // self.spawn_entity(renderer, scenes, record);
    //             // }
    //         }
    //         Event::SpawnDirectionalLight => {
    //             // let key = self.state.nodes.update(|hierarchy| {
    //             //     hierarchy.append(
    //             //         None,
    //             //         node::Node {
    //             //             transform: Transform::default(),
    //             //             name: NodeKind::DirectionalLight.default_name().into(),
    //             //             body: node::NodeBody::DirectionalLight(node::DirectionalLight {
    //             //                 color: Color::WHITE,
    //             //                 illuminance: 100_000.0,
    //             //             }),
    //             //         },
    //             //     )
    //             // });

    //             // let entity = scenes.graph.append(
    //             //     None,
    //             //     game_scene::scene2::Node::from_transform(Transform::default()),
    //             // );
    //             // scenes.spawner.insert(
    //             //     entity,
    //             //     Scene {
    //             //         nodes: Node {
    //             //             transform: Transform::default(),
    //             //             body: NodeBody::DirectionalLight(DirectionalLight {
    //             //                 color: Color::WHITE,
    //             //                 illuminance: 100_000.0,
    //             //             }),
    //             //         }
    //             //         .into(),
    //             //         materials: vec![],
    //             //         meshes: vec![],
    //             //         images: vec![],
    //             //     },
    //             // );

    //             // self.node_map.insert(key, entity);
    //         }
    //         Event::SpawnPointLight => {
    //             // let key = self.state.nodes.update(|hierarchy| {
    //             //     hierarchy.append(
    //             //         None,
    //             //         node::Node {
    //             //             transform: Transform::default(),
    //             //             name: NodeKind::PointLight.default_name().into(),
    //             //             body: node::NodeBody::PointLight(node::PointLight {
    //             //                 color: Color::WHITE,
    //             //                 intensity: 100.0,
    //             //                 radius: 100.0,
    //             //             }),
    //             //         },
    //             //     )
    //             // });

    //             // let entity = scenes.graph.append(
    //             //     None,
    //             //     game_scene::scene2::Node::from_transform(Transform::default()),
    //             // );
    //             // scenes.spawner.insert(
    //             //     entity,
    //             //     Scene {
    //             //         nodes: Node {
    //             //             transform: Transform::default(),
    //             //             body: NodeBody::PointLight(PointLight {
    //             //                 color: Color::WHITE,
    //             //                 intensity: 100.0,
    //             //                 radius: 100.0,
    //             //             }),
    //             //         }
    //             //         .into(),
    //             //         materials: vec![],
    //             //         images: vec![],
    //             //         meshes: vec![],
    //             //     },
    //             // );

    //             // self.node_map.insert(key, entity);
    //         }
    //         Event::SpawnSpotLight => {
    //             // let key = self.state.nodes.update(|hierarchy| {
    //             //     hierarchy.append(
    //             //         None,
    //             //         node::Node {
    //             //             transform: Transform::default(),
    //             //             name: NodeKind::SpotLight.default_name().into(),
    //             //             body: node::NodeBody::SpotLight(node::SpotLight {
    //             //                 color: Color::WHITE,
    //             //                 intensity: 100.0,
    //             //                 radius: 100.0,
    //             //                 inner_cutoff: 45.0,
    //             //                 outer_cutoff: 50.0,
    //             //             }),
    //             //         },
    //             //     )
    //             // });
    //         }
    //         Event::Destroy { node } => {
    //             // FIXME: Removing parent should remove all childrne.

    //             // self.state.nodes.update(|hierarchy| {
    //             //     hierarchy.remove(node);
    //             // });

    //             // if let Some(entity) = self.node_map.remove(&node) {
    //             //     scenes.graph.remove(entity);
    //             // }
    //         }
    //         Event::UpdateTransform { transform } => {
    //             // let nodes = self.state.selection.get();

    //             // for node in nodes {
    //             //     self.state.nodes.update(|hierarchy| {
    //             //         let node = hierarchy.get_mut(node).unwrap();
    //             //         node.transform = transform;
    //             //     });

    //             //     // if let Some(entity) = self.node_map.get(&node) {
    //             //     //     scenes.graph.get_mut(*entity).unwrap().transform = transform;
    //             //     // }

    //             //     self.state.props.update(|props| {
    //             //         props.transform = transform;
    //             //     });
    //             // }
    //         }
    //     }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Axis {
    X,
    Y,
    Z,
}

pub struct State {
    entities: WriteSignal<Vec<Entity>>,
}

pub fn build_ui(cx: &Scope, state: EditorState) -> State {
    let root = cx.append(Container::new());

    let (entities, set_entities) = root.create_signal(Vec::new());

    cx.append(Panel { entities });

    State {
        entities: set_entities,
    }
}

fn build_object_transform(cx: &Scope, props: ReadSignal<NodeProperties>, tx: mpsc::Sender<Event>) {
    let root = cx.append(Container::new());

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
            let props = props.clone();
            let tx = tx.clone();

            move |val| {
                let mut transform = props.with_untracked(|props| props.transform);
                transform.translation.x = val;
                tx.send(Event::UpdateTransform { transform });
            }
        };

        let set_y = {
            let props = props.clone();
            let tx = tx.clone();

            move |val| {
                let mut transform = props.with_untracked(|props| props.transform);
                transform.translation.y = val;
                tx.send(Event::UpdateTransform { transform });
            }
        };

        let set_z = {
            let props = props.clone();
            let tx = tx.clone();

            move |val| {
                let mut transform = props.with_untracked(|props| props.transform);
                transform.translation.z = val;
                tx.send(Event::UpdateTransform { transform });
            }
        };

        let wrapper_style = Style {
            growth: Growth::x(1.0),
            direction: Direction::Column,
            ..Default::default()
        };

        let (translation_x, set_translation_x) = cx.create_signal(0.0);
        let (translation_y, set_translation_y) = cx.create_signal(0.0);
        let (translation_z, set_translation_z) = cx.create_signal(0.0);

        {
            let props = props.clone();
            cx.create_effect(move || {
                let translation = props.with(|props| props.transform.translation);
                set_translation_x.set(translation.x);
                set_translation_y.set(translation.y);
                set_translation_z.set(translation.z);
            });
        }

        let x = translation_row.append(Container::new().style(wrapper_style.clone()));
        x.append(Text::new().text("X".to_owned()));
        x.append(
            ParseInput::new(ValueProvider::Reader(translation_x))
                .style(style.clone())
                .on_change(set_x),
        );

        let y = translation_row.append(Container::new().style(wrapper_style.clone()));
        y.append(Text::new().text("Y".to_owned()));
        y.append(
            ParseInput::new(ValueProvider::Reader(translation_y))
                .style(style.clone())
                .on_change(set_y),
        );

        let z = translation_row.append(Container::new().style(wrapper_style));
        z.append(Text::new().text("Z".to_owned()));
        z.append(
            ParseInput::new(ValueProvider::Reader(translation_z))
                .style(style)
                .on_change(set_z),
        );
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
            let props = props.clone();
            let tx = tx.clone();

            move |val| {
                let mut transform = props.with_untracked(|props| props.transform);
                transform.rotation.x = val;
                tx.send(Event::UpdateTransform { transform });
            }
        };

        let set_y = {
            let props = props.clone();
            let tx = tx.clone();

            move |val| {
                let mut transform = props.with_untracked(|props| props.transform);
                transform.rotation.y = val;
                tx.send(Event::UpdateTransform { transform });
            }
        };

        let set_z = {
            let props = props.clone();
            let tx = tx.clone();

            move |val| {
                let mut transform = props.with_untracked(|props| props.transform);
                transform.rotation.z = val;
                tx.send(Event::UpdateTransform { transform });
            }
        };

        let set_w = {
            move |val| {
                let mut transform = props.with_untracked(|props| props.transform);
                transform.rotation.w = val;
                tx.send(Event::UpdateTransform { transform });
            }
        };

        let x = row.append(Container::new().style(wrapper_style.clone()));
        x.append(Text::new().text("X".to_owned()));
        x.append(ParseInput::new(0.0).on_change(set_x));

        let y = row.append(Container::new().style(wrapper_style.clone()));
        y.append(Text::new().text("Y".to_owned()));
        y.append(ParseInput::new(0.0).on_change(set_y));

        let z = row.append(Container::new().style(wrapper_style.clone()));
        z.append(Text::new().text("Z".to_owned()));
        z.append(ParseInput::new(0.0).on_change(set_z));

        let w = row.append(Container::new().style(wrapper_style));
        w.append(Text::new().text("W".to_owned()));
        w.append(ParseInput::new(1.0).on_change(set_w));
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Event {
    UpdateSelection { node: Key, additive: bool },
    Spawn(RecordReference),
    Destroy { node: Key },
    SpawnDirectionalLight,
    SpawnPointLight,
    SpawnSpotLight,
    UpdateTransform { transform: Transform },
}

#[derive(Clone, Debug, Default)]
pub struct NodeProperties {
    transform: Transform,
}

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
                camera_transform.rotation = camera_transform.rotation.normalize();

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
