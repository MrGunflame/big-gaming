//! An immutable view of a scene.
mod components;
mod edit;
mod node;
mod panel;

use std::sync::mpsc;

use bitflags::bitflags;
use game_common::collections::string::SmallStr;
use game_common::components::components::{Components, RawComponent};
use game_common::components::{Color, PointLight, PrimaryCamera};
use game_common::components::{MeshInstance, Transform};
use game_common::entity::EntityId;
use game_common::world::World;
use game_core::modules::Modules;
use game_input::keyboard::KeyCode;
use game_input::mouse::{MouseButton, MouseMotion, MouseWheel};
use game_input::ButtonState;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::Renderer;
use game_ui::reactive::{Scope, WriteSignal};
use game_ui::style::{Direction, Justify, Style};
use game_ui::widgets::Container;
use game_wasm::world::RecordReference;
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use glam::{Quat, Vec2, Vec3};

use self::components::ComponentsPanel;
use self::edit::{EditMode, EditOperation};
use self::panel::{Entity, Panel};

const ZOOM_DISTANCE_MIN: f32 = 0.2;
const ZOOM_DISTANCE_MAX: f32 = 100.0;
const ZOOM_FACTOR: f32 = 0.15 / 120.0 * 120.0;

pub struct WorldWindowState {
    camera: EntityId,
    camera_controller: CameraController,
    // TODO: Use `Cursor` instead of adding our own thing.
    cursor: Vec2,
    state: State,
    edit_op: EditOperation,
    events: mpsc::Receiver<Event>,
}

impl WorldWindowState {
    pub fn new(cx: &Scope, window_id: WindowId, world: &mut World, modules: Modules) -> Self {
        let (writer, reader) = mpsc::channel();

        let st = build_ui(cx, writer, modules);

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

        st.entities.set(vec![
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
            state: st,
            events: reader,
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

        let camera_transform = world.get_typed::<Transform>(self.camera).unwrap();
        let mut camera = Camera {
            transform: camera_transform,
            projection: Projection::default(),
            target: RenderTarget::Window(window),
        };
        camera.update_aspect_ratio(viewport_size);

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
                        let selected: Vec<_> = self.state.entities.with(|entities| {
                            entities
                                .iter()
                                .filter_map(|entity| entity.is_selected.then_some(entity.id))
                                .collect()
                        });

                        for entity in selected {
                            world.despawn(entity);
                        }
                    }
                    _ => (),
                }

                if event.state.is_pressed()
                    && self
                        .state
                        .entities
                        .with(|entities| entities.iter().any(|e| e.is_selected))
                {
                    match event.key_code {
                        Some(KeyCode::Escape) => {
                            self.cancel_edit_op(world);
                            self.edit_op.set_mode(EditMode::None);
                        }
                        Some(KeyCode::G) => {
                            self.edit_op.set_mode(EditMode::Translate(None));
                            self.start_edit_op(world, camera, viewport_size.as_vec2());
                        }
                        Some(KeyCode::R) => {
                            self.edit_op.set_mode(EditMode::Rotate(None));
                            self.start_edit_op(world, camera, viewport_size.as_vec2());
                        }
                        Some(KeyCode::S) => {
                            self.edit_op.set_mode(EditMode::Scale(None));
                            self.start_edit_op(world, camera, viewport_size.as_vec2());
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
                MouseButton::Left if event.state.is_pressed() => {
                    if self.edit_op.mode() == EditMode::None {
                        // self.update_selection(renderer, scenes, window);
                    } else {
                        self.confirm_edit_op(renderer);
                    }
                }
                MouseButton::Right => {
                    if self.edit_op.mode() != EditMode::None {
                        self.cancel_edit_op(world);
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

        world.insert_typed(self.camera, camera.transform);
    }

    fn start_edit_op(&mut self, world: &mut World, camera: Camera, viewport_size: Vec2) {
        let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

        self.edit_op.create(self.cursor, ray);

        self.state.entities.with(|entities| {
            for entity in entities.iter().filter(|e| e.is_selected) {
                let transform = world.get_typed(entity.id).unwrap();
                self.edit_op.push(entity.id, transform);
            }
        });
    }

    fn update_edit_op(&mut self, world: &mut World, camera: Camera, viewport_size: Vec2) {
        let camera_rotation = camera.transform.rotation;
        let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

        for (entity, transform) in self.edit_op.update(ray, camera_rotation) {
            world.insert_typed(entity, transform);
        }
    }

    fn cancel_edit_op(&mut self, world: &mut World) {
        for (entity, transform) in self.edit_op.reset() {
            world.insert_typed(entity, transform);
        }
    }

    fn confirm_edit_op(&mut self, renderer: &mut Renderer) {
        self.edit_op.set_mode(EditMode::None);
        self.edit_op.confirm();
    }

    pub fn update(&mut self, world: &mut World) {
        let mut update_components_panel = false;

        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Spawn => {
                    // Create new entities at the location the camera is looking at.
                    let id = world.spawn();
                    world.insert_typed(
                        id,
                        Transform::from_translation(self.camera_controller.origin),
                    );

                    self.state.entities.update(|entities| {
                        entities.push(Entity {
                            id,
                            name: SmallStr::from_static("<entity>"),
                            is_selected: false,
                        });
                    });
                }
                Event::SelectEntity(entity) => {
                    self.state.entities.update(|entities| {
                        for ent in entities.iter_mut() {
                            if ent.id == entity {
                                ent.is_selected ^= true;

                                // If the entity changed we may need to update the
                                // components panel, but we don't need to do this
                                // if the entity has not changed.
                                update_components_panel = true;

                                break;
                            }
                        }
                    });
                }
                Event::UpdateComponent(id, component) => {
                    self.state.entities.with(|entities| {
                        for entity in entities.iter().filter(|e| e.is_selected) {
                            world.insert(entity.id, id, component.clone());
                        }
                    });

                    update_components_panel = true;
                }
            }
        }

        if update_components_panel {
            let selected_entities = self
                .state
                .entities
                .get()
                .into_iter()
                .filter(|v| v.is_selected)
                .collect::<Vec<_>>();

            let components = if selected_entities.is_empty() {
                Components::new()
            } else {
                let mut components = world.components(selected_entities[0].id).clone();

                for entity in selected_entities.iter().skip(1) {
                    let other = world.components(entity.id);
                    components = components.intersection(other);
                }

                components
            };

            self.state.components.set(components);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Axis {
    X,
    Y,
    Z,
}

pub struct State {
    entities: WriteSignal<Vec<Entity>>,
    components: WriteSignal<Components>,
}

fn build_ui(cx: &Scope, writer: mpsc::Sender<Event>, modules: Modules) -> State {
    let root = cx.append(Container::new());

    let (entities, set_entities) = root.create_signal(Vec::new());
    let (components, set_components) = root.create_signal(Components::default());

    let style = Style {
        direction: Direction::Column,
        justify: Justify::SpaceBetween,
        ..Default::default()
    };

    let root = cx.append(Container::new().style(style));

    root.append(Panel {
        entities,
        writer: writer.clone(),
    });
    root.append(ComponentsPanel {
        components,
        writer,
        modules,
    });

    State {
        entities: set_entities,
        components: set_components,
    }
}

#[derive(Clone, Debug)]
enum Event {
    Spawn,
    /// Select or unselect entity.
    SelectEntity(EntityId),
    /// Update component on selected entities.
    UpdateComponent(RecordReference, RawComponent),
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
