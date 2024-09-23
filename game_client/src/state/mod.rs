use std::sync::{mpsc, Arc};

use game_common::world::World;
use game_core::counter::UpdateCounter;
use game_core::modules::Modules;
use game_core::time::Time;
use game_render::camera::RenderTarget;
use game_script::Executor;
use game_tracing::trace_span;
use game_ui::reactive::Runtime;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;

use crate::config::Config;
use crate::input::Inputs;
use crate::ui::title_menu::{MenuEvent, MultiPlayerMenu, TitleMenu};
use crate::ui::UiRootContext;
use crate::world::{GameWorldState, RemoteError};

use self::main_menu::MainMenuState;

pub mod main_menu;

#[derive(Debug)]
pub struct GameState {
    inner: GameStateInner,
    tx: mpsc::Sender<MenuEvent>,
    rx: mpsc::Receiver<MenuEvent>,

    ui_ctx: Option<UiRootContext>,

    config: Config,
    modules: Modules,
    inputs: Inputs,
    executor: Executor,
    cursor: Arc<Cursor>,
}

impl GameState {
    pub fn new(
        config: Config,
        modules: Modules,
        inputs: Inputs,
        executor: Executor,
        cursor: Arc<Cursor>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            inner: GameStateInner::Startup,
            tx,
            rx,
            config,
            modules,
            inputs,
            executor,
            cursor,
            ui_ctx: None,
        }
    }

    pub fn connect(&mut self, addr: String) {
        self.inner = GameStateInner::GameWorld(GameWorldState::new(
            &self.config,
            addr,
            self.modules.clone(),
            &self.cursor,
            self.inputs.clone(),
        ));
    }

    pub async fn update(
        &mut self,
        world: &mut World,
        fps_counter: UpdateCounter,
        time: &mut Time,
    ) -> Result<(), UpdateError> {
        let _span = trace_span!("GameState::update").entered();

        let Some(ui_ctx) = &mut self.ui_ctx else {
            return Ok(());
        };

        while let Ok(event) = self.rx.try_recv() {
            match event {
                MenuEvent::Connect(addr) => {
                    ui_ctx.clear();
                    self.connect(addr);
                    return Ok(());
                }
                MenuEvent::Exit => return Err(UpdateError::Exit),
                MenuEvent::SpawnMainMenu => {
                    ui_ctx.clear();
                    ui_ctx.append(TitleMenu {
                        events: self.tx.clone(),
                    });
                }
                MenuEvent::SpawnMultiPlayerMenu => {
                    ui_ctx.clear();
                    ui_ctx.append(MultiPlayerMenu {
                        events: self.tx.clone(),
                    });
                }
            }
        }

        match &mut self.inner {
            GameStateInner::Startup => {
                self.inner = GameStateInner::MainMenu(MainMenuState::new(world));

                ui_ctx.append(TitleMenu {
                    events: self.tx.clone(),
                });
            }
            GameStateInner::GameWorld(state) => {
                match state
                    .update(world, ui_ctx, fps_counter, &mut self.executor)
                    .await
                {
                    Ok(()) => {}
                    Err(RemoteError::Disconnected) => {
                        self.inner = GameStateInner::Startup;
                    }
                }
            }
            GameStateInner::MainMenu(state) => {
                state.update(time, world);
            }
            _ => (),
        }

        Ok(())
    }

    pub fn handle_event(&mut self, event: WindowEvent, cursor: &Cursor, ui_rt: &Runtime) {
        match event {
            WindowEvent::WindowCreated(event) => {
                let document = ui_rt
                    .create_document(RenderTarget::Window(event.window))
                    .unwrap();
                self.ui_ctx = Some(UiRootContext::new(document, ui_rt.clone()));
            }
            _ => (),
        }

        match &mut self.inner {
            GameStateInner::GameWorld(state) => {
                let Some(ui_ctx) = &mut self.ui_ctx else {
                    return;
                };

                state.handle_event(event, cursor, ui_ctx);
            }
            _ => (),
        }
    }
}

#[derive(Debug, Default)]
enum GameStateInner {
    /// Initial game startup phase.
    #[default]
    Startup,
    MainMenu(MainMenuState),
    /// Connecting to server
    Connecting,
    /// Connection failed
    ConnectionFailure,
    /// Connected to game world.
    GameWorld(GameWorldState),
}

#[derive(Copy, Clone, Debug)]
pub enum UpdateError {
    Exit,
}
