use std::sync::{mpsc, Arc};

use game_common::world::World;
use game_core::counter::UpdateCounter;
use game_core::modules::Modules;
use game_core::time::Time;
use game_render::camera::RenderTarget;
use game_script::Executor;
use game_tracing::trace_span;
use game_ui::runtime::Runtime;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;

use crate::config::Config;
use crate::input::Inputs;
use crate::ui::startup::Startup;
use crate::ui::title_menu::{MenuEvent, TitleMenu};
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
    cursor: Arc<Cursor>,
}

impl GameState {
    pub fn new(config: Config, cursor: Arc<Cursor>) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            inner: GameStateInner::Startup,
            tx,
            rx,
            config,
            cursor,
            ui_ctx: None,
        }
    }

    pub fn init(&mut self, modules: Modules, inputs: Inputs, executor: Executor) {
        if let Some(ui_ctx) = &mut self.ui_ctx {
            ui_ctx.clear();
        }

        self.inner = GameStateInner::Init(InitState {
            modules,
            inputs,
            executor,
            inner: InitStateInner::Startup,
        });
    }

    pub fn connect(&mut self, addr: String) {
        if let GameStateInner::Init(state) = &mut self.inner {
            let world = GameWorldState::new(
                &self.config,
                addr,
                state.modules.clone(),
                &self.cursor,
                state.inputs.clone(),
            );

            match world {
                Ok(world) => state.inner = InitStateInner::GameWorld(world),
                Err(RemoteError::Error(err)) => {
                    tracing::error!("connection error: {}", err);
                    state.inner = InitStateInner::ConnectionFailure(err.to_string())
                }
                Err(RemoteError::Disconnected) => {
                    unreachable!()
                }
            }
        }
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
            }
        }

        match &mut self.inner {
            GameStateInner::Startup => {}
            GameStateInner::Init(init_state) => match &mut init_state.inner {
                InitStateInner::Startup => {
                    init_state.inner = InitStateInner::MainMenu(MainMenuState::new(world));

                    ui_ctx.append(TitleMenu {
                        events: self.tx.clone(),
                    });
                }
                InitStateInner::GameWorld(state) => {
                    match state
                        .update(world, ui_ctx, fps_counter, &mut init_state.executor)
                        .await
                    {
                        Ok(()) => {}
                        Err(RemoteError::Disconnected) => {
                            init_state.inner = InitStateInner::Startup;
                        }
                        Err(RemoteError::Error(err)) => {
                            tracing::error!("connection error: {}", err);
                        }
                    }
                }
                InitStateInner::MainMenu(state) => {
                    state.update(time, world);
                }
                InitStateInner::ConnectionFailure(err) => {
                    init_state.inner = InitStateInner::Startup;
                }
                _ => (),
            },
        }

        Ok(())
    }

    pub fn handle_event(&mut self, event: WindowEvent, cursor: &Cursor, ui_rt: &Runtime) {
        match event {
            WindowEvent::WindowCreated(event) => {
                let document = ui_rt
                    .create_document(RenderTarget::Window(event.window))
                    .unwrap();
                let mut ui_ctx = UiRootContext::new(document, ui_rt.clone());

                if let GameStateInner::Startup = self.inner {
                    ui_ctx.append(Startup {});
                }

                self.ui_ctx = Some(ui_ctx);
            }
            _ => (),
        }

        match &mut self.inner {
            GameStateInner::Init(state) => match &mut state.inner {
                InitStateInner::GameWorld(state) => {
                    let Some(ui_ctx) = &mut self.ui_ctx else {
                        return;
                    };

                    state.handle_event(event, cursor, ui_ctx);
                }
                _ => (),
            },
            _ => (),
        }
    }
}

#[derive(Debug, Default)]
enum GameStateInner {
    /// Initial game startup phase.
    #[default]
    Startup,
    Init(InitState),
}

#[derive(Debug)]
struct InitState {
    modules: Modules,
    inputs: Inputs,
    executor: Executor,
    inner: InitStateInner,
}

#[derive(Debug)]
enum InitStateInner {
    Startup,
    MainMenu(MainMenuState),
    /// Connecting to server
    Connecting,
    /// Connection failed
    ConnectionFailure(String),
    /// Connected to game world.
    GameWorld(GameWorldState),
}

#[derive(Copy, Clone, Debug)]
pub enum UpdateError {
    Exit,
}
