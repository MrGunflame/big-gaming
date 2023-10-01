mod actions;
pub mod camera;
pub mod movement;

use std::net::ToSocketAddrs;
use std::sync::Arc;

use ahash::HashMap;
use game_common::components::actor::ActorProperties;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::module::ModuleId;
use game_common::record::RecordReference;
use game_common::world::entity::EntityBody;
use game_core::counter::Interval;
use game_core::hierarchy::{Entity, TransformHierarchy};
use game_core::modules::Modules;
use game_core::time::Time;
use game_data::record::Record;
use game_input::hotkeys::{HotkeyCode, HotkeyKind, Key};
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseMotion;
use game_net::message::{DataMessageBody, EntityAction, EntityRotate, EntityTranslate};
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::color::Color;
use game_render::entities::CameraId;
use game_render::light::DirectionalLight;
use game_render::Renderer;
use game_scene::Scenes;
use game_script::executor::ScriptExecutor;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::WindowState;
use glam::Vec3;

use crate::config::Config;
use crate::entities::actor::SpawnActor;
use crate::entities::object::SpawnObject;
use crate::entities::terrain::spawn_terrain;
use crate::input::{InputKey, Inputs};
use crate::net::world::{Command, CommandBuffer, DelayedEntity};
use crate::net::ServerConnection;
use crate::utils::extract_actor_rotation;

use self::actions::ActiveActions;
use self::camera::{CameraController, CameraMode};
use self::movement::update_rotation;

#[derive(Debug)]
pub struct GameWorldState {
    pub conn: ServerConnection<Interval>,
    camera_controller: CameraController,
    is_init: bool,
    primary_camera: Option<CameraId>,
    entities: HashMap<EntityId, Entity>,
    modules: Modules,
    actions: ActiveActions,
    executor: Arc<ScriptExecutor>,
    inputs: Inputs,
}

impl GameWorldState {
    pub fn new(
        config: &Config,
        addr: impl ToSocketAddrs,
        modules: Modules,
        cursor: &Cursor,
        executor: Arc<ScriptExecutor>,
        inputs: Inputs,
    ) -> Self {
        cursor.lock();
        cursor.set_visible(false);

        let mut conn = ServerConnection::new(config);
        conn.connect(addr);

        Self {
            conn,
            camera_controller: CameraController::new(),
            is_init: false,
            primary_camera: None,
            entities: HashMap::default(),
            modules,
            actions: ActiveActions::new(),
            executor,
            inputs,
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        window: WindowState,
        time: &Time,
        hierarchy: &mut TransformHierarchy,
    ) {
        if !self.is_init {
            self.is_init = true;

            let camera = Camera {
                transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                projection: Projection::default(),
                target: RenderTarget::Window(window.id()),
            };

            self.primary_camera = Some(renderer.entities.cameras.insert(camera));

            renderer
                .entities
                .directional_lights
                .insert(DirectionalLight {
                    transform: Transform::default(),
                    color: Color::WHITE,
                    illuminance: 100_000.0,
                });
        }

        let mut buf = CommandBuffer::new();
        self.conn.update(time, &mut buf, &self.executor);

        while let Some(cmd) = buf.pop() {
            match cmd {
                Command::Spawn(entity) => {
                    let eid = entity.entity.id;

                    if entity.host {
                        self.update_host(eid);
                    }

                    if let Some(id) =
                        spawn_entity(renderer, scenes, entity, &self.modules, hierarchy)
                    {
                        self.entities.insert(eid, id);
                    }
                }
                Command::Translate {
                    entity,
                    start,
                    end,
                    dst,
                } => {
                    let id = self.entities.get(&entity).unwrap();
                    let transform = hierarchy.get_mut(*id).unwrap();

                    tracing::trace!(
                        "translate entity {:?} from {:?} to {:?}",
                        entity,
                        transform.translation,
                        dst
                    );

                    transform.translation = dst;
                }
                Command::Rotate {
                    entity,
                    start,
                    end,
                    dst,
                } => {
                    let id = self.entities.get(&entity).unwrap();
                    let transform = hierarchy.get_mut(*id).unwrap();

                    tracing::trace!(
                        "rotate entity {:?} from {:?} to {:?}",
                        entity,
                        transform.rotation,
                        dst
                    );

                    transform.rotation = dst;
                }
                Command::SpawnHost(id) => {
                    self.update_host(id);
                }
                _ => todo!(),
            }
        }

        self.dispatch_actions();

        if let Some(snapshot) = &self.conn.current_state {
            if let Some(entity) = snapshot.entities.get(self.conn.host) {
                let props = ActorProperties {
                    eyes: Vec3::new(0.0, 1.8, 0.0),
                    rotation: extract_actor_rotation(entity.transform.rotation),
                };

                self.camera_controller
                    .sync_with_entity(entity.transform, props);
            }
        }

        if let Some(id) = self.primary_camera {
            let mut camera = renderer.entities.cameras.get_mut(id).unwrap();
            camera.transform = self.camera_controller.transform;
        }
    }

    pub fn handle_event(&mut self, event: WindowEvent, cursor: &Cursor) {
        match event {
            WindowEvent::MouseMotion(event) => {
                self.handle_mouse_motion(event);
            }
            WindowEvent::KeyboardInput(event) => {
                self.handle_keyboard_input(event, cursor);
            }
            WindowEvent::MouseButtonInput(event) => {
                self.actions.send_mouse_event(event);
            }
            _ => (),
        }
    }

    fn handle_mouse_motion(&mut self, event: MouseMotion) {
        if let Some(snapshot) = &mut self.conn.current_state {
            if let Some(host) = snapshot.entities.get_mut(self.conn.host) {
                host.transform = update_rotation(host.transform, event);
                let rotation = host.transform.rotation;

                let entity = self.conn.server_entities.get(self.conn.host).unwrap();
                self.conn.send(DataMessageBody::EntityRotate(EntityRotate {
                    entity,
                    rotation,
                }));
            }
        }
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInput, cursor: &Cursor) {
        self.actions.send_keyboard_event(event);

        if !event.state.is_pressed() {
            return;
        }

        match event.key_code {
            Some(KeyCode::Escape) => {
                if event.state.is_pressed() {
                    if cursor.is_locked() {
                        cursor.unlock();
                        cursor.set_visible(true);
                    } else {
                        cursor.lock();
                        cursor.set_visible(false);
                    }
                }
            }
            // FIXME: Temporary, move translation to scripts instead.
            Some(KeyCode::W) => self.update_translation(-Vec3::Z),
            Some(KeyCode::S) => self.update_translation(Vec3::Z),
            Some(KeyCode::A) => self.update_translation(-Vec3::X),
            Some(KeyCode::D) => self.update_translation(Vec3::X),
            Some(KeyCode::V) => match self.camera_controller.mode {
                CameraMode::FirstPerson => {
                    self.camera_controller.mode = CameraMode::ThirdPerson { distance: 5.0 }
                }
                CameraMode::ThirdPerson { distance } => {
                    self.camera_controller.mode = CameraMode::FirstPerson;
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn update_translation(&mut self, dir: Vec3) {
        if let Some(snapshot) = &mut self.conn.current_state {
            if let Some(host) = snapshot.entities.get_mut(self.conn.host) {
                host.transform.translation += host.transform.rotation * dir * 0.01;
                let translation = host.transform.translation;

                let entity = self.conn.server_entities.get(self.conn.host).unwrap();
                self.conn
                    .send(DataMessageBody::EntityTranslate(EntityTranslate {
                        entity,
                        translation,
                    }));
            }
        }
    }

    fn dispatch_actions(&mut self) {
        let actions = self.actions.take_events();

        let Some(entity) = self.conn.server_entities.get(self.conn.host) else {
            return;
        };

        for action in actions {
            self.conn.send(DataMessageBody::EntityAction(EntityAction {
                entity,
                action,
            }));
        }
    }

    fn update_host(&mut self, id: EntityId) {
        // TODO: Unregister previous host.
        self.conn.host = id;

        let snapshot = self.conn.current_state.as_ref().unwrap();
        let entity = snapshot.entities.get(id).unwrap();
        let actor = entity.body.as_actor().unwrap();

        let module = self.modules.get(actor.race.0.module).unwrap();
        let record = module.records.get(actor.race.0.record).unwrap();
        let race = record.body.as_race().unwrap();

        for action in &race.actions {
            let module = self.modules.get(action.module).unwrap();
            let record = module.records.get(action.record).unwrap();

            self.actions.register(
                action.module,
                record,
                self.get_key_for_action(action.module, record),
            );
        }
    }

    fn run_scripts(&mut self) {}

    fn get_key_for_action(&self, module: ModuleId, record: &Record) -> Key {
        let input = self
            .inputs
            .inputs
            .get(&RecordReference {
                module,
                record: record.id,
            })
            .unwrap();

        let key = match input.input_keys[0] {
            InputKey::KeyCode(key) => HotkeyCode::KeyCode { key_code: key },
            InputKey::ScanCode(key) => HotkeyCode::ScanCode { scan_code: key },
        };

        Key {
            trigger: input.trigger,
            code: key,
        }
    }
}

fn spawn_entity(
    renderer: &mut Renderer,
    scenes: &mut Scenes,
    entity: DelayedEntity,
    modules: &Modules,
    hierarchy: &mut TransformHierarchy,
) -> Option<Entity> {
    // TODO: Check if can spawn an entity before allocating one.
    let root = hierarchy.append(None, entity.entity.transform);

    match entity.entity.body {
        EntityBody::Terrain(terrain) => {
            spawn_terrain(scenes, renderer, &terrain.mesh, root);
        }
        EntityBody::Object(object) => SpawnObject {
            id: object.id,
            entity: root,
        }
        .spawn(scenes, modules),
        EntityBody::Actor(actor) => SpawnActor {
            race: actor.race,
            transform: entity.entity.transform,
            entity: root,
        }
        .spawn(scenes, modules),
        EntityBody::Item(item) => todo!(),
    }

    Some(root)
}
