mod actions;
pub mod camera;
pub mod movement;

use std::net::ToSocketAddrs;

use ahash::HashMap;
use game_common::components::actor::ActorProperties;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::world::entity::EntityBody;
use game_core::counter::Interval;
use game_core::modules::Modules;
use game_core::time::Time;
use game_input::keyboard::KeyboardInput;
use game_input::mouse::MouseMotion;
use game_net::message::{DataMessageBody, EntityAction, EntityRotate, EntityTranslate};
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::color::Color;
use game_render::entities::CameraId;
use game_render::light::DirectionalLight;
use game_render::Renderer;
use game_scene::{SceneId, Scenes};
use game_window::cursor::Cursor;
use game_window::events::{VirtualKeyCode, WindowEvent};
use game_window::windows::WindowState;
use glam::Vec3;

use crate::config::Config;
use crate::entities::actor::SpawnActor;
use crate::entities::object::SpawnObject;
use crate::entities::terrain::spawn_terrain;
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
    entities: HashMap<EntityId, SceneId>,
    modules: Modules,
    actions: ActiveActions,
}

impl GameWorldState {
    pub fn new(
        config: &Config,
        addr: impl ToSocketAddrs,
        modules: Modules,
        cursor: &Cursor,
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
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        window: WindowState,
        time: &Time,
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
        self.conn.update(time, &mut buf);

        while let Some(cmd) = buf.pop() {
            match cmd {
                Command::Spawn(entity) => {
                    let eid = entity.entity.id;
                    if let Some(id) = spawn_entity(renderer, scenes, entity, &self.modules) {
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
                    let mut transform = scenes.get_transform(*id).unwrap();
                    transform.translation = dst;
                    scenes.set_transform(*id, transform);
                }
                Command::Rotate {
                    entity,
                    start,
                    end,
                    dst,
                } => {
                    let id = self.entities.get(&entity).unwrap();
                    let mut transform = scenes.get_transform(*id).unwrap();
                    transform.rotation = dst;
                    scenes.set_transform(*id, transform);
                }
                Command::SpawnHost(id) => {}
                _ => todo!(),
            }
        }

        self.dispatch_actions();

        {
            let host = self.conn.host;
            if let Some(id) = self.entities.get(&host) {
                if let Some(transform) = scenes.get_transform(*id) {
                    let props = ActorProperties {
                        eyes: Vec3::new(0.0, 1.8, 0.0),
                        rotation: extract_actor_rotation(transform.rotation),
                    };
                    self.camera_controller.sync_with_entity(transform, props);
                }
            }
        }

        if let Some(id) = self.primary_camera {
            let mut camera = renderer.entities.cameras.get_mut(id).unwrap();
            camera.transform = self.camera_controller.transform;
        }
    }

    pub fn handle_event(&mut self, scenes: &mut Scenes, event: WindowEvent, cursor: &Cursor) {
        match event {
            WindowEvent::MouseMotion(event) => {
                self.handle_mouse_motion(scenes, event);
            }
            WindowEvent::KeyboardInput(event) => {
                self.handle_keyboard_input(scenes, event, cursor);
            }
            WindowEvent::MouseButtonInput(event) => {
                self.actions.send_mouse_event(event);
            }
            _ => (),
        }
    }

    fn handle_mouse_motion(&mut self, scenes: &mut Scenes, event: MouseMotion) {
        if let Some(id) = self.entities.get(&self.conn.host) {
            if let Some(mut transform) = scenes.get_transform(*id) {
                transform = update_rotation(transform, event);

                let entity = self.conn.server_entities.get(self.conn.host).unwrap();
                self.conn.send(DataMessageBody::EntityRotate(EntityRotate {
                    entity,
                    rotation: transform.rotation,
                }));
            }
        }
    }

    fn handle_keyboard_input(
        &mut self,
        scenes: &mut Scenes,
        event: KeyboardInput,
        cursor: &Cursor,
    ) {
        self.actions.send_keyboard_event(event);

        if !event.state.is_pressed() {
            return;
        }

        match event.key_code {
            Some(VirtualKeyCode::Escape) => {
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
            Some(VirtualKeyCode::W) => self.update_translation(scenes, -Vec3::Z),
            Some(VirtualKeyCode::S) => self.update_translation(scenes, Vec3::Z),
            Some(VirtualKeyCode::A) => self.update_translation(scenes, -Vec3::X),
            Some(VirtualKeyCode::D) => self.update_translation(scenes, Vec3::X),
            Some(VirtualKeyCode::V) => match self.camera_controller.mode {
                CameraMode::FirstPerson => {
                    dbg!("sw");
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

    fn update_translation(&mut self, scenes: &mut Scenes, dir: Vec3) {
        if let Some(id) = self.entities.get(&self.conn.host) {
            if let Some(mut transform) = scenes.get_transform(*id) {
                transform.translation += transform.rotation * dir * 0.01;

                let entity = self.conn.server_entities.get(self.conn.host).unwrap();
                self.conn
                    .send(DataMessageBody::EntityTranslate(EntityTranslate {
                        entity,
                        translation: transform.translation,
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
}

fn spawn_entity(
    renderer: &mut Renderer,
    scenes: &mut Scenes,
    entity: DelayedEntity,
    modules: &Modules,
) -> Option<SceneId> {
    match entity.entity.body {
        EntityBody::Terrain(terrain) => Some(spawn_terrain(
            scenes,
            renderer,
            &terrain.mesh,
            entity.entity.transform,
        )),
        EntityBody::Object(object) => SpawnObject {
            id: object.id,
            transform: entity.entity.transform,
        }
        .spawn(scenes, modules),
        EntityBody::Actor(actor) => SpawnActor {
            race: actor.race,
            transform: entity.entity.transform,
        }
        .spawn(scenes, modules),
        EntityBody::Item(item) => todo!(),
    }
}
