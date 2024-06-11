mod actions;
pub mod camera;
pub mod game_world;
pub mod movement;
pub mod script;
pub mod state;

use std::net::ToSocketAddrs;
use std::sync::mpsc;
use std::time::Duration;

use game_common::components::actions::ActionId;
use game_common::components::{GlobalTransform, PrimaryCamera, Transform};
use game_common::entity::EntityId;
use game_common::module::ModuleId;
use game_common::record::RecordReference;
use game_common::world::World;
use game_core::counter::{Interval, UpdateCounter};
use game_core::modules::Modules;
use game_core::time::Time;
use game_data::record::Record;
use game_input::hotkeys::{HotkeyCode, Key};
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseMotion;
use game_script::Executor;
use game_ui::reactive::{Document, NodeId};
use game_wasm::encoding::BinaryWriter;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;

use crate::components::base::{Camera, Health};
use crate::config::Config;
// use crate::entities::actor::SpawnActor;
// use crate::entities::object::SpawnObject;
// use crate::entities::terrain::spawn_terrain;
use crate::input::{InputKey, Inputs};
use crate::net::world::{Command, CommandBuffer};
use crate::net::{connect_udp, ServerConnection};
use crate::ui::debug::Statistics;
use crate::ui::inventory::InventoryProxy;
use crate::ui::main_menu::MainMenu;
use crate::ui::{UiElements, UiEvent};

use self::actions::ActiveActions;
use self::camera::{CameraController, CameraMode, DetachedState};
use self::game_world::{Action, GameWorld};
use self::movement::update_rotation;

#[derive(Clone, Debug)]
pub enum RemoteError {
    Disconnected,
}

#[derive(Debug)]
pub struct GameWorldState {
    pub world: GameWorld,
    camera_controller: CameraController,
    primary_camera: Option<EntityId>,
    modules: Modules,
    actions: ActiveActions,
    inputs: Inputs,
    inventory_proxy: Option<InventoryProxy>,
    registered_actions: Vec<ActionId>,
    main_menu: Option<NodeId>,
    cursor_pinned: CursorPinState,
    host: EntityId,
    ui_elements: UiElements,
    interval: Interval,
    ui_events_rx: mpsc::Receiver<UiEvent>,
    // Keep the sender around so we can clone
    // and send it to the UI elements for callbacks.
    ui_events_tx: mpsc::Sender<UiEvent>,
}

impl GameWorldState {
    pub fn new(
        config: &Config,
        addr: impl ToSocketAddrs,
        modules: Modules,
        cursor: &Cursor,
        executor: Executor,
        inputs: Inputs,
    ) -> Self {
        let mut cursor_pinned = CursorPinState::new();
        if cursor.window().is_some() {
            cursor_pinned.pin(cursor);
        }

        let handle = connect_udp(addr).unwrap();
        let conn = ServerConnection::new(handle);

        let interval = Interval::new(Duration::from_secs(1) / config.timestep);

        let (ui_events_tx, ui_events_rx) = mpsc::channel();

        let mut this = Self {
            world: GameWorld::new(conn, executor, config),
            camera_controller: CameraController::new(),
            primary_camera: None,
            modules,
            actions: ActiveActions::new(),
            inputs,
            inventory_proxy: None,
            registered_actions: vec![],
            host: EntityId::dangling(),
            main_menu: None,
            cursor_pinned,
            ui_elements: UiElements::default(),
            interval,
            ui_events_rx,
            ui_events_tx,
        };
        this.register_actions();
        this
    }

    pub async fn update(
        &mut self,
        time: &Time,
        world: &mut World,
        ui_doc: &Document,
        fps_counter: UpdateCounter,
    ) -> Result<(), RemoteError> {
        self.interval.wait(time.last_update()).await;

        let mut buf = CommandBuffer::new();
        self.world.update(&self.modules, &mut buf)?;

        *world = self.world.state().world.clone();

        self.primary_camera = Some(world.spawn());
        world.insert_typed(self.primary_camera.unwrap(), GlobalTransform::default());
        world.insert_typed(self.primary_camera.unwrap(), PrimaryCamera);

        while let Some(cmd) = buf.pop() {
            match cmd {
                Command::SpawnHost(id) => {
                    self.update_host(id);
                }
            }
        }

        let mut cx = ui_doc.root_scope();

        // Debug stats
        self.ui_elements.update_debug_state(
            &mut cx,
            Some(Statistics {
                ups: self.world.ups(),
                fps: fps_counter,
                entities: world.len() as u64,
                net_input_buffer_len: self.world.input_buffer_len() as u64,
                rtt: self.world.rtt(),
            }),
        );

        // Health
        if let Ok(camera) = world.get_typed::<Camera>(self.host) {
            if let Ok(health) = world.get_typed::<Health>(camera.parent.into()) {
                self.ui_elements
                    .update_health(&mut cx, Some(health), &self.ui_events_tx);
            } else {
                self.ui_elements
                    .update_health(&mut cx, None, &self.ui_events_tx);
            }
        }

        self.dispatch_actions();

        if self.camera_controller.mode != CameraMode::Detached {
            if self.world.state().world.contains(self.host) {
                let transform: Transform = self.world.state().world.get_typed(self.host).unwrap();
                self.camera_controller.transform = transform;
            }
        } else {
            // We are in detached mode and need to manually
            // check if we are moving.
            self.camera_controller.update();
        }

        if let Some(id) = self.primary_camera {
            world.insert_typed(id, GlobalTransform(self.camera_controller.transform));
        }

        Ok(())
    }

    pub fn handle_event(&mut self, event: WindowEvent, cursor: &Cursor, ui_doc: &Document) {
        match event {
            WindowEvent::MouseMotion(event) => {
                self.handle_mouse_motion(event);
            }
            WindowEvent::KeyboardInput(event) => {
                self.handle_keyboard_input(event, cursor, ui_doc);
            }
            WindowEvent::MouseButtonInput(event) => {
                self.actions.send_mouse_event(event);
            }
            WindowEvent::CursorLeft(event) => {
                if !self.cursor_pinned.is_pinned() {
                    return;
                }

                let cx = ui_doc.root_scope();
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

        if !self.world.state().world.contains(self.host) {
            return;
        }

        let mut transform = self
            .world
            .state()
            .world
            .get_typed::<Transform>(self.host)
            .unwrap();
        transform = update_rotation(transform, event);
        // We must update the rotation, otherwise following mouse motion events
        // will get overwritten by previous events in the same frame.
        self.world
            .state_mut()
            .world
            .insert_typed(self.host, transform);
        self.camera_controller.transform = transform;

        let (_, data) = BinaryWriter::new().encoded(&transform.rotation);

        self.world.send(Action {
            entity: self.host,
            action: ActionId("c626b9b0ab1940aba6932ea7726d0175:23".parse().unwrap()),
            data,
        });
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInput, cursor: &Cursor, ui_doc: &Document) {
        match event.key_code {
            Some(KeyCode::Escape) if event.state.is_pressed() => {
                match self.main_menu {
                    Some(id) => {
                        ui_doc.root_scope().remove(id);
                        self.main_menu = None;
                        self.cursor_pinned.pin(cursor);
                    }
                    None => {
                        let cx = ui_doc.root_scope();
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
                    ui_doc.root_scope().remove(pxy.id);
                    self.inventory_proxy = None;
                    self.cursor_pinned.pin(cursor);
                }
                None => {
                    let camera: Camera = self.world.state().world.get_typed(self.host).unwrap();

                    // Ignore if the current player entity has no inventory.
                    let Ok(inventory) = self
                        .world
                        .state()
                        .world
                        .get_typed::<game_wasm::inventory::Inventory>(EntityId::from_raw(
                            camera.parent.into_raw(),
                        ))
                    else {
                        return;
                    };

                    self.inventory_proxy = Some(InventoryProxy::new(
                        &inventory,
                        self.modules.clone(),
                        ui_doc,
                        self.ui_events_tx.clone(),
                    ));
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

        if !self.world.state().world.contains(self.host) {
            return;
        }

        for action in actions {
            tracing::debug!("emit action {:?}", action);

            self.world.send(Action {
                entity: self.host,
                action,
                data: vec![],
            });
        }

        while let Ok(event) = self.ui_events_rx.try_recv() {
            self.world.send(Action {
                entity: self.host,
                action: ActionId(event.id),
                data: event.data,
            });
        }
    }

    fn update_host(&mut self, id: EntityId) {
        self.host = id;
    }

    fn register_actions(&mut self) {
        for module in self.modules.iter() {
            for record in module.records.iter() {
                let Some(action) = record.body.as_action() else {
                    continue;
                };

                if let Some(key) = self.get_key_for_action(module.id, record) {
                    self.actions.register(module.id, record, key);
                    self.registered_actions.push(ActionId(RecordReference {
                        module: module.id,
                        record: record.id,
                    }));
                }
            }
        }
    }

    fn get_key_for_action(&self, module: ModuleId, record: &Record) -> Option<Key> {
        let input = self.inputs.inputs.get(&RecordReference {
            module,
            record: record.id,
        })?;

        let key = match input.input_keys[0] {
            InputKey::KeyCode(key) => HotkeyCode::KeyCode { key_code: key },
            InputKey::ScanCode(key) => HotkeyCode::ScanCode { scan_code: key },
        };

        Some(Key {
            trigger: input.trigger,
            code: key,
        })
    }
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
