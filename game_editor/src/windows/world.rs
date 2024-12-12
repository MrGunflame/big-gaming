pub mod components;
pub mod edit;
pub mod entity_hierarchy;
pub mod node;
pub mod properties;

use std::collections::VecDeque;

use bitflags::bitflags;
use edit::Axis;
use game_common::collections::string::SmallStr;
use game_common::components::components::{Components, RawComponent};
use game_common::components::Transform;
use game_common::components::{GlobalTransform, PrimaryCamera};
use game_common::entity::EntityId;
use game_common::world::hierarchy::update_global_transform;
use game_common::world::World;
use game_input::keyboard::KeyCode;
use game_input::mouse::{MouseButton, MouseMotion, MouseWheel};
use game_input::ButtonState;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::entities::SceneId;
use game_render::options::{MainPassOptions, ShadingMode};
use game_render::Renderer;
use game_ui::widgets::Callback;
use game_wasm::world::RecordReference;
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use glam::{Quat, Vec2, Vec3};

use self::edit::{EditMode, EditOperation};

const ZOOM_DISTANCE_MIN: f32 = 0.2;
const ZOOM_DISTANCE_MAX: f32 = 100.0;
const ZOOM_FACTOR: f32 = 0.15 / 120.0 * 120.0;

#[derive(Clone, Debug)]
pub enum WorldEvent {
    UpdateTransform(EntityId, Transform),
}

#[derive(Debug, Default)]
struct WorldState {
    world: World,
    entities: Vec<Entity>,
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: EntityId,
    pub name: SmallStr,
    pub is_selected: bool,
}

pub struct WorldWindowState {
    camera_controller: CameraController,
    // TODO: Use `Cursor` instead of adding our own thing.
    cursor: Vec2,
    edit_op: EditOperation,
    rendering_properties: RenderingProperties,
    state: WorldState,
    events: VecDeque<WorldEvent>,
}

impl WorldWindowState {
    pub fn new() -> Self {
        Self {
            camera_controller: CameraController::default(),
            cursor: Vec2::ZERO,
            edit_op: EditOperation::new(),
            rendering_properties: RenderingProperties::default(),
            state: WorldState::default(),
            events: VecDeque::new(),
        }
    }

    pub fn handle_event(
        &mut self,
        event: WindowEvent,
        window: WindowId,
        renderer: &mut Renderer,
        scene_id: SceneId,
    ) {
        let viewport_size = renderer.get_surface_size(window.into()).unwrap();

        let mut camera = Camera {
            transform: self.camera_controller.transform,
            projection: Projection::default(),
            target: RenderTarget::Window(window),
            scene: scene_id,
        };
        camera.update_aspect_ratio(viewport_size);

        match event {
            WindowEvent::MouseMotion(event) => {
                self.camera_controller.update(event);
            }
            WindowEvent::MouseWheel(event) => {
                self.camera_controller.zoom(event);
            }
            WindowEvent::CursorLeft(_) => {
                // Reset the mode when the cursor leaves the window.
                self.camera_controller.mode = Mode::NONE;
            }
            WindowEvent::CursorMoved(event) => {
                self.cursor = event.position;

                if self.edit_op.mode() != EditMode::None {
                    self.update_edit_op(camera, viewport_size.as_vec2());
                }
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
                        let mut delete_entities = Vec::new();
                        self.state.entities.retain(|entity| {
                            if entity.is_selected {
                                delete_entities.push(entity.id);
                                false
                            } else {
                                true
                            }
                        });

                        for entity in delete_entities {
                            self.state.world.despawn(entity);
                        }
                    }
                    _ => (),
                }

                if event.state.is_pressed() && self.state.entities.iter().any(|e| e.is_selected) {
                    match event.key_code {
                        Some(KeyCode::Escape) => {
                            if self.edit_op.mode() != EditMode::None {
                                self.cancel_edit_op();
                                self.edit_op.set_mode(EditMode::None);
                            }
                        }
                        Some(KeyCode::G) => {
                            self.edit_op.set_mode(EditMode::Translate(None));
                            self.start_edit_op(camera, viewport_size.as_vec2());
                        }
                        Some(KeyCode::R) => {
                            self.edit_op.set_mode(EditMode::Rotate(None));
                            self.start_edit_op(camera, viewport_size.as_vec2());
                        }
                        Some(KeyCode::S) => {
                            self.edit_op.set_mode(EditMode::Scale(None));
                            self.start_edit_op(camera, viewport_size.as_vec2());
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
                                self.update_edit_op(camera, viewport_size.as_vec2());
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
                                self.update_edit_op(camera, viewport_size.as_vec2());
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
                                self.update_edit_op(camera, viewport_size.as_vec2());
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
                        self.confirm_edit_op();
                    }
                }
                MouseButton::Right => {
                    if self.edit_op.mode() != EditMode::None {
                        self.cancel_edit_op();
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
    }

    fn start_edit_op(&mut self, camera: Camera, viewport_size: Vec2) {
        let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

        self.edit_op.create(self.cursor, ray);

        for entity in self.state.entities.iter().filter(|e| e.is_selected) {
            let transform = self.state.world.get_typed(entity.id).unwrap();
            self.edit_op.push(entity.id, transform);
        }
    }

    fn update_edit_op(&mut self, camera: Camera, viewport_size: Vec2) {
        debug_assert!(self.edit_op.mode() != EditMode::None);

        let camera_rotation = camera.transform.rotation;
        let ray = camera.viewport_to_world(camera.transform, viewport_size, self.cursor);

        for (entity, transform) in self.edit_op.update(ray, camera_rotation) {
            self.state.world.insert_typed(entity, transform);
        }
        update_global_transform(&mut self.state.world);
    }

    fn cancel_edit_op(&mut self) {
        debug_assert!(self.edit_op.mode() != EditMode::None);

        for (entity, transform) in self.edit_op.reset() {
            self.state.world.insert_typed(entity, transform);
        }
        update_global_transform(&mut self.state.world);
    }

    fn confirm_edit_op(&mut self) {
        for (entity, transform) in self.edit_op.confirm() {
            self.events
                .push_back(WorldEvent::UpdateTransform(entity, transform));
        }
    }

    pub fn update(&mut self, world: &mut World, options: &mut MainPassOptions) {
        {
            *world = self.state.world.clone();
            let camera = world.spawn();
            world.insert_typed(camera, self.camera_controller.transform);
            world.insert_typed(camera, GlobalTransform(self.camera_controller.transform));
            world.insert_typed(camera, PrimaryCamera);
        }

        options.shading = self.rendering_properties.shading;
    }

    pub fn set_shading_mode(&mut self, mode: ShadingMode) {
        self.rendering_properties.shading = mode;
    }

    pub fn toggle_selection(&mut self, entity: EntityId) -> bool {
        for e in self.state.entities.iter_mut() {
            if e.id == entity {
                e.is_selected ^= true;
                return true;
            }
        }

        false
    }

    pub fn insert_component_on_entity(
        &mut self,
        entity: EntityId,
        id: RecordReference,
        component: RawComponent,
    ) {
        self.state.world.insert(entity, id, component);
    }

    pub fn insert_component(&mut self, id: RecordReference, component: RawComponent) {
        for e in self.state.entities.iter().filter(|e| e.is_selected) {
            self.state.world.insert(e.id, id, component.clone());
        }
    }

    pub fn remove_component(&mut self, id: RecordReference) {
        for e in self.state.entities.iter().filter(|e| e.is_selected) {
            self.state.world.remove(e.id, id);
        }
    }

    pub fn entities(&self) -> Vec<Entity> {
        self.state.entities.clone()
    }

    pub fn spawn(&mut self) -> EntityId {
        let id = self.state.world.spawn();
        self.state.world.insert_typed(id, Transform::default());
        self.state.entities.push(Entity {
            id,
            name: "entity".into(),
            is_selected: false,
        });
        update_global_transform(&mut self.state.world);
        id
    }

    pub fn despawn(&mut self, entity: EntityId) {
        self.state.world.despawn(entity);
        self.state.entities.retain(|e| e.id != entity);
    }

    pub fn spawn_world(&mut self, world: World) -> EntityId {
        let id = self.state.world.append(world);
        self.state.world.insert_typed(id, Transform::default());
        self.state.entities.push(Entity {
            id,
            name: "entity".into(),
            is_selected: false,
        });
        update_global_transform(&mut self.state.world);
        id
    }

    pub fn components(&self, entity: EntityId) -> Components {
        self.state.world.components(entity).clone()
    }

    pub fn pop_event(&mut self) -> Option<WorldEvent> {
        self.events.pop_front()
    }

    pub fn world(&self) -> &World {
        &self.state.world
    }
}

#[derive(Debug, Default)]
pub struct SceneState {
    pub entities: Vec<Entity>,
    pub entities_changed: Callback<()>,
    pub components: Components,
    pub components_changed: Callback<()>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Spawn,
    /// Select or unselect entity.
    SelectEntity(EntityId),
    /// Update component on selected entities.
    UpdateComponent(RecordReference, RawComponent),
    DeleteComponent(RecordReference),
    SetShadingMode(ShadingMode),
    DespawnEntity(EntityId),
}

#[derive(Clone, Debug, Default)]
struct CameraController {
    origin: Vec3,
    mode: Mode,
    transform: Transform,
}

impl CameraController {
    fn new(transform: Transform) -> Self {
        Self {
            origin: Vec3::ZERO,
            mode: Mode::empty(),
            transform,
        }
    }

    fn zoom(&mut self, event: MouseWheel) {
        let distance = (self.transform.translation - self.origin).length();
        let factor = ZOOM_FACTOR * event.y;
        let offset = f32::clamp(
            distance * (1.0 + factor),
            ZOOM_DISTANCE_MIN,
            ZOOM_DISTANCE_MAX,
        );

        let dir = self.transform.rotation * -Vec3::Z;
        self.transform.translation = self.origin - (dir * offset);
    }

    fn update(&mut self, event: MouseMotion) {
        match self.mode {
            mode if mode == Mode::TRANSLATE => {
                let x = -event.delta.x * 0.01;
                let y = event.delta.y * 0.01;

                let mut distance = (self.transform.rotation * Vec3::X) * x;
                distance += (self.transform.rotation * Vec3::Y) * y;

                self.transform.translation += distance;
                self.origin += distance;
            }
            mode if mode == Mode::ROTATE => {
                let x = event.delta.x * 0.01;
                let y = event.delta.y * 0.01;

                // // Rotate around origin with a constant distance.
                let distance = (self.transform.translation - self.origin).length().abs();

                let q1 = Quat::from_axis_angle(Vec3::Y, -x);
                let q2 = Quat::from_axis_angle(Vec3::X, -y);

                self.transform.rotation = q1 * self.transform.rotation;
                self.transform.rotation = self.transform.rotation * q2;

                // Renormalize quat due to FP error creep.
                self.transform.rotation = self.transform.rotation.normalize();

                // FIXME: FP error creep means that distance will very slowly grow
                // over time. Storing the radius instead of computing the distance
                // would fix this.
                self.transform.translation =
                    self.origin + self.transform.rotation * Vec3::new(0.0, 0.0, distance);
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

#[derive(Clone, Debug)]
pub struct OnWorldChangeEvent {
    pub world: World,
    pub entities: Vec<EntityId>,
}

#[derive(Clone, Debug, Default)]
pub struct RenderingProperties {
    pub shading: ShadingMode,
}
