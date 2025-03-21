mod actions;
pub mod camera;
pub mod game_world;
pub mod movement;
pub mod script;
pub mod state;

use std::net::ToSocketAddrs;
use std::sync::mpsc;

use game_common::components::actions::ActionId;
use game_common::components::{GlobalTransform, PrimaryCamera, RigidBody, Transform};
use game_common::entity::EntityId;
use game_common::world::World;
use game_core::counter::UpdateCounter;
use game_core::modules::Modules;
use game_input::hotkeys::{HotkeyCode, Key};
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseMotion;
use game_script::Executor;
use game_ui::runtime::NodeId;
use game_wasm::encoding::BinaryWriter;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;

use crate::components::base::{Camera, Health, ACTION_ROTATE};
use crate::config::Config;
use crate::input::{InputKey, Inputs};
use crate::net::world::{Command, CommandBuffer};
use crate::net::{connect_udp, ConnectionError, ServerConnection};
use crate::ui::debug::{PlayerInfo, Statistics};
use crate::ui::inventory::InventoryProxy;
use crate::ui::main_menu::MainMenu;
use crate::ui::{UiElements, UiEvent, UiRootContext};

use self::actions::ActiveActions;
use self::camera::{CameraController, CameraMode, DetachedState};
use self::game_world::{Action, GameWorld};
use self::movement::update_rotation;

#[derive(Debug)]
pub enum RemoteError {
    Disconnected,
    Error(ConnectionError),
}

#[derive(Debug)]
pub struct GameWorldState {
    world: GameWorld,
    camera_controller: CameraController,
    primary_camera: Option<EntityId>,
    modules: Modules,
    actions: ActiveActions,
    inputs: Inputs,
    inventory_proxy: Option<InventoryProxy>,
    main_menu: Option<NodeId>,
    cursor_pinned: CursorPinState,
    host: EntityId,
    ui_elements: UiElements,
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
        inputs: Inputs,
    ) -> Result<Self, RemoteError> {
        let handle = connect_udp(addr).map_err(RemoteError::Error)?;
        let conn = ServerConnection::new(handle);

        let (ui_events_tx, ui_events_rx) = mpsc::channel();

        let mut cursor_pinned = CursorPinState::new();
        if cursor.window().is_some() {
            cursor_pinned.pin(cursor);
        }

        let mut this = Self {
            world: GameWorld::new(conn, config),
            camera_controller: CameraController::new(),
            primary_camera: None,
            modules,
            actions: ActiveActions::new(),
            inputs,
            inventory_proxy: None,
            host: EntityId::dangling(),
            main_menu: None,
            cursor_pinned,
            ui_elements: UiElements::default(),
            ui_events_rx,
            ui_events_tx,
        };
        this.register_actions();
        Ok(this)
    }

    pub async fn update(
        &mut self,
        world: &mut World,
        ui_ctx: &mut UiRootContext,
        fps_counter: UpdateCounter,
        executor: &mut Executor,
    ) -> Result<(), RemoteError> {
        let mut buf = CommandBuffer::new();
        self.world.update(&self.modules, executor, &mut buf).await?;

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

        let mut player_info = PlayerInfo::default();
        if let Ok(camera) = world.get_typed::<Camera>(self.host) {
            // Health
            if let Ok(health) = world.get_typed::<Health>(camera.parent) {
                self.ui_elements
                    .update_health(ui_ctx, Some(health), &self.ui_events_tx);
            } else {
                self.ui_elements
                    .update_health(ui_ctx, None, &self.ui_events_tx);
            }

            if let Ok(transform) = world.get_typed::<Transform>(camera.parent) {
                player_info.transform = Some(transform);
            }

            if let Ok(rigid_body) = world.get_typed::<RigidBody>(camera.parent) {
                player_info.rigid_body = Some(rigid_body);
            }
        }

        // Debug stats
        self.ui_elements.update_debug_state(
            ui_ctx,
            Some(Statistics {
                ups: self.world.statistics().ups.clone(),
                fps: fps_counter,
                entities: world.len() as u64,
                net_input_buffer_len: self.world.statistics().input_buffer_len,
                rtt: self.world.statistics().rtt,
                player_info,
            }),
        );

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

    pub fn handle_event(
        &mut self,
        event: WindowEvent,
        cursor: &Cursor,
        ui_ctx: &mut UiRootContext,
    ) {
        match event {
            WindowEvent::MouseMotion(event) => {
                self.handle_mouse_motion(event);
            }
            WindowEvent::KeyboardInput(event) => {
                self.handle_keyboard_input(event, cursor, ui_ctx);
            }
            WindowEvent::MouseButtonInput(event) => {
                self.actions.send_mouse_event(event);
            }
            WindowEvent::CursorLeft(event) => {
                if !self.cursor_pinned.is_pinned() {
                    return;
                }

                self.main_menu = Some(ui_ctx.append(MainMenu {}).node().unwrap());
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

        let Ok(mut transform) = self.world.state().world.get_typed::<Transform>(self.host) else {
            return;
        };

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
            action: ActionId(ACTION_ROTATE),
            data,
        });
    }

    fn handle_keyboard_input(
        &mut self,
        event: KeyboardInput,
        cursor: &Cursor,
        ui_ctx: &mut UiRootContext,
    ) {
        match event.key_code {
            Some(KeyCode::Escape) if event.state.is_pressed() => {
                match self.main_menu {
                    Some(id) => {
                        ui_ctx.remove(id);
                        self.main_menu = None;
                        self.cursor_pinned.pin(cursor);
                    }
                    None => {
                        self.main_menu = Some(ui_ctx.append(MainMenu {}).node().unwrap());
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
                    ui_ctx.remove(pxy.id);
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
                        ui_ctx,
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
        for (id, input) in &self.inputs.inputs {
            let key = match input.input_keys[0] {
                InputKey::KeyCode(key) => HotkeyCode::KeyCode { key_code: key },
                InputKey::ScanCode(key) => HotkeyCode::ScanCode { scan_code: key },
            };

            self.actions.register(
                id.module,
                id.record,
                Key {
                    trigger: input.trigger,
                    code: key,
                },
            );
        }
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
