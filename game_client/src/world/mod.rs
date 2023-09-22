pub mod camera;
pub mod movement;

use std::net::ToSocketAddrs;

use ahash::{HashMap, HashSet};
use game_common::components::actor::ActorProperties;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::world::entity::EntityBody;
use game_core::counter::Interval;
use game_core::hierarchy::Entity;
use game_core::time::Time;
use game_input::keyboard::KeyboardInput;
use game_input::mouse::MouseMotion;
use game_net::message::{DataMessageBody, EntityRotate};
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::color::Color;
use game_render::entities::CameraId;
use game_render::light::DirectionalLight;
use game_render::Renderer;
use game_scene::{SceneId, Scenes};
use game_window::events::{VirtualKeyCode, WindowEvent};
use game_window::windows::WindowId;
use glam::Vec3;

use crate::config::Config;
use crate::entities::actor::spawn_actor;
use crate::entities::object::{spawn_object, SpawnObject};
use crate::entities::terrain::spawn_terrain;
use crate::net::world::{Command, CommandBuffer, DelayedEntity};
use crate::net::ServerConnection;
use crate::utils::extract_actor_rotation;

use self::camera::CameraController;
use self::movement::update_rotation;

#[derive(Debug)]
pub struct GameWorldState {
    pub conn: ServerConnection<Interval>,
    camera_controller: CameraController,
    is_init: bool,
    primary_camera: Option<CameraId>,
    entities: HashMap<EntityId, SceneId>,
}

impl GameWorldState {
    pub fn new(config: &Config, addr: impl ToSocketAddrs) -> Self {
        let mut conn = ServerConnection::new(config);
        conn.connect(addr);

        Self {
            conn,
            camera_controller: CameraController::new(),
            is_init: false,
            primary_camera: None,
            entities: HashMap::default(),
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        window: WindowId,
        time: &Time,
    ) {
        if !self.is_init {
            self.is_init = true;

            let camera = Camera {
                transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                projection: Projection::default(),
                target: RenderTarget::Window(window),
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
                    let id = spawn_entity(renderer, scenes, entity);
                    self.entities.insert(eid, id);
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
                _ => todo!(),
            }
        }

        let host = self.conn.host;
        if let Some(view) = self.conn.world.back() {
            if let Some(host) = view.get(host) {
                let props = ActorProperties {
                    eyes: Vec3::new(0.0, 0.0, 1.8),
                    rotation: extract_actor_rotation(host.transform.rotation),
                };
                self.camera_controller
                    .sync_with_entity(host.transform, props);
            }
        }

        if let Some(id) = self.primary_camera {
            let mut camera = renderer.entities.cameras.get_mut(id).unwrap();
            camera.transform = self.camera_controller.transform;
        }
    }

    pub fn handle_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::MouseMotion(event) => {
                self.handle_mouse_motion(event);
            }
            WindowEvent::KeyboardInput(event) => {
                self.handle_keyboard_input(event);
            }
            _ => (),
        }
    }

    fn handle_mouse_motion(&mut self, event: MouseMotion) {
        let Some(mut view) = self.conn.world.back_mut() else {
            return;
        };

        if let Some(mut host) = view.get_mut(self.conn.host) {
            let transform = update_rotation(host.transform, event);
            host.set_translation(transform.translation);
            host.set_rotation(transform.rotation);
            drop(view);

            let entity = self.conn.server_entities.get(self.conn.host).unwrap();

            self.conn.send(DataMessageBody::EntityRotate(EntityRotate {
                entity,
                rotation: transform.rotation,
            }));
        }
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInput) {
        match event.key_code {
            Some(VirtualKeyCode::Escape) => {}
            _ => (),
        }
    }
}

fn spawn_entity(renderer: &mut Renderer, scenes: &mut Scenes, entity: DelayedEntity) -> SceneId {
    match entity.entity.body {
        EntityBody::Terrain(terrain) => {
            spawn_terrain(scenes, renderer, &terrain.mesh, entity.entity.transform)
        }
        EntityBody::Object(object) => spawn_object(
            scenes,
            renderer,
            SpawnObject {
                id: object.id,
                transform: entity.entity.transform,
            },
        ),
        EntityBody::Actor(actor) => spawn_actor(scenes),
        EntityBody::Item(item) => todo!(),
    }
}
