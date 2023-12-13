mod actions;
pub mod camera;
pub mod game_world;
pub mod movement;
pub mod script;
pub mod state;

use std::net::ToSocketAddrs;
use std::time::Duration;

use ahash::HashMap;
use game_common::components::actions::ActionId;
use game_common::components::actor::ActorProperties;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::module::ModuleId;
use game_common::record::RecordReference;
use game_common::world::entity::EntityBody;
use game_core::counter::Interval;
use game_core::hierarchy::TransformHierarchy;
use game_core::modules::Modules;
use game_core::time::Time;
use game_data::record::{Record, RecordBody};
use game_input::hotkeys::{HotkeyCode, Key};
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseMotion;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::color::Color;
use game_render::entities::CameraId;
use game_render::light::DirectionalLight;
use game_render::Renderer;
use game_scene::scene2::{self, Node};
use game_script::executor::ScriptExecutor;
use game_ui::reactive::NodeId;
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
use crate::net::world::{Command, CommandBuffer};
use crate::net::ServerConnection;
use crate::scene::SceneState;
use crate::ui::inventory::InventoryProxy;
use crate::ui::main_menu::MainMenu;
use crate::utils::extract_actor_rotation;

use self::actions::ActiveActions;
use self::camera::{CameraController, CameraMode, DetachedState};
use self::game_world::{GameWorld, SendCommand};
use self::movement::update_rotation;

#[derive(Debug)]
pub struct GameWorldState {
    pub world: GameWorld<Interval>,
    camera_controller: CameraController,
    is_init: bool,
    primary_camera: Option<CameraId>,
    entities: HashMap<EntityId, scene2::Key>,
    modules: Modules,
    actions: ActiveActions,
    inputs: Inputs,
    inventory_proxy: Option<InventoryProxy>,
    inventory_actions: Vec<ActionId>,
    main_menu: Option<NodeId>,
    cursor_pinned: CursorPinState,
    host: EntityId,
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
        let mut cursor_pinned = CursorPinState::new();
        if cursor.window().is_some() {
            cursor_pinned.pin(cursor);
        }

        let mut conn = ServerConnection::new();
        conn.connect(addr);

        let interval = Interval::new(Duration::from_secs(1) / config.timestep);

        Self {
            world: GameWorld::new(conn, interval, executor, config),
            camera_controller: CameraController::new(),
            is_init: false,
            primary_camera: None,
            entities: HashMap::default(),
            modules,
            actions: ActiveActions::new(),
            inputs,
            inventory_proxy: None,
            inventory_actions: vec![],
            host: EntityId::dangling(),
            main_menu: None,
            cursor_pinned,
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        scenes: &mut SceneState,
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
                    transform: Transform {
                        translation: Vec3::splat(100.0),
                        ..Default::default()
                    }
                    .looking_at(Vec3::splat(0.0), Vec3::Y),
                    color: Color::WHITE,
                    illuminance: 100_000.0,
                });
        }

        let mut buf = CommandBuffer::new();
        self.world.update(time, &self.modules, &mut buf);

        while let Some(cmd) = buf.pop() {
            match cmd {
                Command::Spawn(entity) => {
                    let eid = entity.id;

                    if let Some(id) =
                        spawn_entity(renderer, scenes, entity, &self.modules, hierarchy)
                    {
                        self.entities.insert(eid, id);
                    }
                }
                Command::Despawn(id) => {
                    let key = self.entities.remove(&id).unwrap();
                    scenes.graph.remove(key);
                }
                Command::Translate { entity, dst } => {
                    let key = self.entities.get(&entity).unwrap();
                    let node = scenes.graph.get_mut(*key).unwrap();

                    tracing::trace!(
                        "translate entity {:?} from {:?} to {:?}",
                        entity,
                        node.transform.translation,
                        dst
                    );

                    node.transform.translation = dst;
                }
                Command::Rotate { entity, dst } => {
                    let key = self.entities.get(&entity).unwrap();
                    let node = scenes.graph.get_mut(*key).unwrap();

                    tracing::trace!(
                        "rotate entity {:?} from {:?} to {:?}",
                        entity,
                        node.transform.rotation,
                        dst
                    );

                    node.transform.rotation = dst;
                }
                Command::SpawnHost(id) => {
                    self.update_host(id);
                }
                Command::ComponentAdd { entity, component } => {}
                Command::ComponentRemove { entity, component } => {}
                Command::InventoryItemEquip { entity, slot } => {
                    if entity == self.host {
                        self.update_inventory_actions();
                    }
                }
                Command::InventoryItemUnequip { entity, slot } => {
                    if entity == self.host {
                        self.update_inventory_actions();
                    }
                }
            }
        }

        self.dispatch_actions();

        if self.camera_controller.mode != CameraMode::Detached {
            if let Some(entity) = self.world.state().entities.get(self.host) {
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
            WindowEvent::CursorLeft(event) => {
                if !self.cursor_pinned.is_pinned() {
                    return;
                }

                let cx = ui_state.get_mut(window).unwrap().root_scope();
                self.main_menu = Some(cx.append(MainMenu {}).id().unwrap());
                self.cursor_pinned.unpin(cursor);
            }
            _ => (),
        }
    }

    fn handle_mouse_motion(&mut self, event: MouseMotion) {
        // Don't control the camera if the cursor is not pinned
        // (e.g. when it is in a UI window).
        if !self.cursor_pinned.is_pinned() {
            return;
        }

        // If the camera is in detached mode, control it directly.
        if self.camera_controller.mode == CameraMode::Detached {
            self.camera_controller.transform =
                update_rotation(self.camera_controller.transform, event);

            return;
        }

        if let Some(host) = self.world.state().entities.get(self.host) {
            let transform = update_rotation(host.transform, event);
            let rotation = transform.rotation;

            self.world.send(SendCommand::Rotate {
                entity: self.host,
                rotation,
            });
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
            Some(KeyCode::Escape) if event.state.is_pressed() => {
                match self.main_menu {
                    Some(id) => {
                        state.get_mut(window).unwrap().root_scope().remove(id);
                        self.main_menu = None;
                        self.cursor_pinned.pin(cursor);
                    }
                    None => {
                        let cx = state.get_mut(window).unwrap().root_scope();
                        self.main_menu = Some(cx.append(MainMenu {}).id().unwrap());
                        self.cursor_pinned.unpin(cursor);
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
                    self.cursor_pinned.pin(cursor);
                }
                None => {
                    let doc = state.get_mut(window).unwrap();

                    // Ignore if the current player entity has no inventory.
                    let Some(inventory) = self.world.state().inventories.get(self.host) else {
                        return;
                    };

                    self.inventory_proxy =
                        Some(InventoryProxy::new(inventory, self.modules.clone(), doc));
                    self.cursor_pinned.unpin(cursor);
                }
            },
            _ => (),
        }

        // UI consumes the event.
        if !self.cursor_pinned.is_pinned() {
            return;
        }

        // Only trigger an action if we didn't already "consume"
        // the input.
        self.actions.send_keyboard_event(event);
    }

    fn dispatch_actions(&mut self) {
        let actions = self.actions.take_events();

        if self.world.state().entities.get(self.host).is_none() {
            return;
        }

        for action in actions {
            self.world.send(SendCommand::Action {
                entity: self.host,
                action,
            });
        }
    }

    fn update_host(&mut self, id: EntityId) {
        // Remove all registered actions from the previous host.
        // If this is the first host this is a noop.
        self.actions.clear();

        self.host = id;

        let entity = self.world.state().entities.get(id).unwrap();
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
        if let Some(inventory) = self.world.state().inventories.get(self.host) {
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

        if let Some(inventory) = self.world.state().inventories.get(self.host) {
            for (_, stack) in inventory.clone().iter() {
                if !stack.item.equipped {
                    continue;
                }

                self.register_record_action(stack.item.id.0);

                for (id, _) in stack.item.components.iter() {
                    self.register_record_action(id);
                }
            }
        }
    }

    fn register_record_action(&mut self, id: RecordReference) {
        let module = self.modules.get(id.module).unwrap();
        let record = module.records.get(id.record).unwrap();

        let actions = match &record.body {
            RecordBody::Action(_) => return,
            RecordBody::Race(race) => &race.actions,
            RecordBody::Component(component) => &component.actions,
            RecordBody::Item(item) => &item.actions,
            RecordBody::Object(_) => return,
        };

        for action in actions {
            let module = self.modules.get(action.module).unwrap();
            let record = module.records.get(action.record).unwrap();

            self.actions.register(
                action.module,
                record,
                self.get_key_for_action(action.module, record),
            );

            self.inventory_actions.push(ActionId(*action));
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
    scenes: &mut SceneState,
    entity: game_common::world::entity::Entity,
    modules: &Modules,
    hierarchy: &mut TransformHierarchy,
) -> Option<scene2::Key> {
    // TODO: Check if can spawn an entity before allocating one.
    let root = scenes
        .graph
        .append(None, Node::from_transform(Transform::default()));

    match entity.body {
        EntityBody::Terrain(terrain) => {
            spawn_terrain(scenes, renderer, &terrain.mesh, root);
        }
        EntityBody::Object(object) => SpawnObject {
            id: object.id,
            key: root,
        }
        .spawn(scenes, modules),
        EntityBody::Actor(actor) => SpawnActor {
            race: actor.race,
            transform: entity.transform,
            key: root,
        }
        .spawn(scenes, modules),
        EntityBody::Item(item) => todo!(),
    }

    Some(root)
}

#[derive(Clone, Debug)]
struct CursorPinState {
    /// Whether the cursor is pinned (locked) in the current window.
    pinned: bool,
}

impl CursorPinState {
    pub fn new() -> Self {
        Self { pinned: false }
    }

    pub fn pin(&mut self, cursor: &Cursor) {
        cursor.lock();
        cursor.set_visible(false);
        self.pinned = true;
    }

    pub fn unpin(&mut self, cursor: &Cursor) {
        cursor.unlock();
        cursor.set_visible(true);
        self.pinned = false;
    }

    pub fn is_pinned(&self) -> bool {
        self.pinned
    }
}
