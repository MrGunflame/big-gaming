mod actions;
pub mod camera;
pub mod game_world;
pub mod movement;
pub mod script;
pub mod state;

use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use ahash::HashMap;
use game_common::components::actions::ActionId;
use game_common::components::actor::ActorProperties;
use game_common::components::items::ItemId;
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
use game_input::hotkeys::{HotkeyCode, Key};
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseMotion;
use game_net::message::{DataMessageBody, EntityAction, EntityRotate};
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::color::Color;
use game_render::entities::CameraId;
use game_render::light::DirectionalLight;
use game_render::Renderer;
use game_scene::Scenes;
use game_script::executor::ScriptExecutor;
use game_ui::UiState;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowId, WindowState};
use glam::Vec3;

use crate::config::Config;
use crate::entities::actor::SpawnActor;
use crate::entities::object::SpawnObject;
use crate::entities::terrain::spawn_terrain;
use crate::input::{InputKey, Inputs};
use crate::net::world::{Command, CommandBuffer, DelayedEntity};
use crate::net::ServerConnection;
use crate::ui::inventory::InventoryProxy;
use crate::utils::extract_actor_rotation;

use self::actions::ActiveActions;
use self::camera::{CameraController, CameraMode, DetachedState};
use self::game_world::GameWorld;
use self::movement::update_rotation;

#[derive(Debug)]
pub struct GameWorldState {
    pub world: GameWorld<Interval>,
    camera_controller: CameraController,
    is_init: bool,
    primary_camera: Option<CameraId>,
    entities: HashMap<EntityId, Entity>,
    modules: Modules,
    actions: ActiveActions,
    executor: Arc<ScriptExecutor>,
    inputs: Inputs,
    inventory_proxy: Option<InventoryProxy>,
    inventory_actions: Vec<ActionId>,
}

impl GameWorldState {
    pub fn new(
        config: &Config,
        addr: impl ToSocketAddrs,
        modules: Modules,
        cursor: &Cursor,
        executor: ScriptExecutor,
        inputs: Inputs,
    ) -> Self {
        cursor.lock();
        cursor.set_visible(false);

        let mut conn = ServerConnection::new(config);
        conn.connect(addr);
        conn.modules = modules.clone();

        let interval = Interval::new(Duration::from_secs(1) / config.timestep);

        Self {
            world: GameWorld::new(conn, interval, executor),
            camera_controller: CameraController::new(),
            is_init: false,
            primary_camera: None,
            entities: HashMap::default(),
            modules,
            actions: ActiveActions::new(),
            executor,
            inputs,
            inventory_proxy: None,
            inventory_actions: vec![],
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut Scenes,
        window: WindowState,
        time: &Time,
        hierarchy: &mut TransformHierarchy,
        ui_state: &mut UiState,
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
                Command::Translate { entity, dst } => {
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
                Command::Rotate { entity, dst } => {
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

        if self.conn.inventory_update {
            self.conn.inventory_update = false;
            self.update_inventory_actions();
        }

        self.dispatch_actions();

        if self.camera_controller.mode != CameraMode::Detached {
            if let Some(entity) = self.world_state.entities.get(self.conn.host) {
                let props = ActorProperties {
                    eyes: Vec3::new(0.0, 1.8, 0.0),
                    rotation: extract_actor_rotation(entity.transform.rotation),
                };

                self.camera_controller
                    .sync_with_entity(entity.transform, props);
            }
        } else {
            // We are in detached mode and need to manually
            // check if we are moving.
            const SPEED: f32 = 0.1;

            if self.camera_controller.detached_state.forward {
                self.camera_controller.transform.translation +=
                    self.camera_controller.transform.rotation * -Vec3::Z * SPEED;
            }

            if self.camera_controller.detached_state.back {
                self.camera_controller.transform.translation +=
                    self.camera_controller.transform.rotation * Vec3::Z * SPEED;
            }

            if self.camera_controller.detached_state.left {
                self.camera_controller.transform.translation +=
                    self.camera_controller.transform.rotation * -Vec3::X * SPEED;
            }

            if self.camera_controller.detached_state.right {
                self.camera_controller.transform.translation +=
                    self.camera_controller.transform.rotation * Vec3::X * SPEED;
            }
        }

        if let Some(id) = self.primary_camera {
            let mut camera = renderer.entities.cameras.get_mut(id).unwrap();
            camera.transform = self.camera_controller.transform;
        }
    }

    pub fn handle_event(
        &mut self,
        event: WindowEvent,
        cursor: &Cursor,
        ui_state: &mut UiState,
        window: WindowId,
    ) {
        match event {
            WindowEvent::MouseMotion(event) => {
                self.handle_mouse_motion(event);
            }
            WindowEvent::KeyboardInput(event) => {
                self.handle_keyboard_input(event, cursor, ui_state, window);
            }
            WindowEvent::MouseButtonInput(event) => {
                self.actions.send_mouse_event(event);
            }
            _ => (),
        }
    }

    fn handle_mouse_motion(&mut self, event: MouseMotion) {
        // If the camera is in detached mode, control it directly.
        if self.camera_controller.mode == CameraMode::Detached {
            self.camera_controller.transform =
                update_rotation(self.camera_controller.transform, event);

            return;
        }

        if let Some(host) = self.world_state.entities.get_mut(self.conn.host) {
            host.transform = update_rotation(host.transform, event);
            let rotation = host.transform.rotation;

            let entity = self.conn.server_entities.get(self.conn.host).unwrap();
            self.conn.send(DataMessageBody::EntityRotate(EntityRotate {
                entity,
                rotation,
            }));
        }
    }

    fn handle_keyboard_input(
        &mut self,
        event: KeyboardInput,
        cursor: &Cursor,
        state: &mut UiState,
        window: WindowId,
    ) {
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

                return;
            }
            Some(KeyCode::V) if event.state.is_pressed() => match self.camera_controller.mode {
                CameraMode::FirstPerson => {
                    self.camera_controller.mode = CameraMode::ThirdPerson { distance: 5.0 };
                    return;
                }
                CameraMode::ThirdPerson { distance } => {
                    self.camera_controller.mode = CameraMode::FirstPerson;
                    return;
                }
                _ => (),
            },
            // Hardcoded controls for detached camera mode.
            // FIXME: Optimally we'd like to integrate these with the default
            // movement hotkeys, but they come from a module and are implemented
            // as an action, which makes this process non-trivial.
            Some(KeyCode::W) if self.camera_controller.mode == CameraMode::Detached => {
                self.camera_controller.detached_state.forward = event.state.is_pressed();
                return;
            }
            Some(KeyCode::S) if self.camera_controller.mode == CameraMode::Detached => {
                self.camera_controller.detached_state.back = event.state.is_pressed();
                return;
            }
            Some(KeyCode::A) if self.camera_controller.mode == CameraMode::Detached => {
                self.camera_controller.detached_state.left = event.state.is_pressed();
                return;
            }
            Some(KeyCode::D) if self.camera_controller.mode == CameraMode::Detached => {
                self.camera_controller.detached_state.right = event.state.is_pressed();
                return;
            }
            // Toggle to go into detached camera mode.
            Some(KeyCode::Tab) if event.state.is_pressed() => {
                self.camera_controller.mode = match self.camera_controller.mode {
                    CameraMode::Detached => CameraMode::FirstPerson,
                    CameraMode::FirstPerson | CameraMode::ThirdPerson { distance: _ } => {
                        CameraMode::Detached
                    }
                };

                self.camera_controller.detached_state = DetachedState::default();
                return;
            }
            Some(KeyCode::I) if event.state.is_pressed() => match &mut self.inventory_proxy {
                Some(pxy) => {
                    state.get_mut(window).unwrap().root_scope().remove(pxy.id);
                    self.inventory_proxy = None;
                }
                None => {
                    let doc = state.get_mut(window).unwrap();

                    // Ignore if the current player entity has no inventory.
                    let Some(inventory) = self.world_state.inventories.get(self.conn.host) else {
                        return;
                    };

                    self.inventory_proxy =
                        Some(InventoryProxy::new(inventory, self.modules.clone(), doc));
                }
            },
            _ => (),
        }

        // Only trigger an action if we didn't already "consume"
        // the input.
        self.actions.send_keyboard_event(event);
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
        // Remove all registered actions from the previous host.
        // If this is the first host this is a noop.
        self.actions.clear();

        self.conn.host = id;

        let entity = self.world_state.entities.get(id).unwrap();
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

        // Register all actions from equipped items.
        if let Some(inventory) = self.world_state.inventories.get(self.conn.host) {
            for (_, stack) in inventory.iter() {
                if !stack.item.equipped {
                    continue;
                }

                let module = self.modules.get(stack.item.id.0.module).unwrap();
                let record = module.records.get(stack.item.id.0.record).unwrap();
                let item = record.body.clone().unwrap_item();

                for action in item.actions {
                    let module = self.modules.get(action.module).unwrap();
                    let record = module.records.get(action.record).unwrap();

                    self.actions.register(
                        action.module,
                        record,
                        self.get_key_for_action(action.module, record),
                    );
                }
            }
        }
    }

    fn update_inventory_actions(&mut self) {
        // This is a quick-and-dirty implementation that throws out all previous
        // inventory item actions and registers all equipped all items once again
        // every time the inventory is updated.

        // Unregister all actions.
        for id in self.inventory_actions.drain(..) {
            let module = self.modules.get(id.0.module).unwrap();
            let record = module.records.get(id.0.record).unwrap();
            self.actions.unregister(id.0.module, record);
        }

        if let Some(inventory) = self.world_state.inventories.get(self.conn.host) {
            for (_, stack) in inventory.clone().iter() {
                if !stack.item.equipped {
                    continue;
                }

                self.register_item_actions(stack.item.id);
            }
        }
    }

    fn register_item_actions(&mut self, id: ItemId) {
        let module = self.modules.get(id.0.module).unwrap();
        let record = module.records.get(id.0.record).unwrap();
        let item = record.body.clone().unwrap_item();

        for action in item.actions {
            let module = self.modules.get(action.module).unwrap();
            let record = module.records.get(action.record).unwrap();

            self.actions.register(
                action.module,
                record,
                self.get_key_for_action(action.module, record),
            );

            self.inventory_actions.push(ActionId(action));
        }
    }

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
