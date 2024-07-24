use std::sync::{mpsc, Arc};

use game_common::world::World;
use game_core::counter::UpdateCounter;
use game_core::modules::Modules;
use game_core::time::Time;
use game_script::Executor;
use game_tracing::trace_span;
use game_ui::reactive::{Context, DocumentId, Runtime};
use game_ui::widgets::Widget;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;

use crate::config::Config;
use crate::input::Inputs;
use crate::ui::title_menu::{MenuEvent, MultiPlayerMenu, TitleMenu};
use crate::world::{GameWorldState, RemoteError};

use self::main_menu::MainMenuState;

pub mod main_menu;

#[derive(Debug)]
pub struct GameState {
    inner: GameStateInner,
    tx: mpsc::Sender<MenuEvent>,
    rx: mpsc::Receiver<MenuEvent>,
    root_ctx: Option<Context<()>>,

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
            root_ctx: None,
            config,
            modules,
            inputs,
            executor,
            cursor,
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
        ui_rt: &Runtime,
        doc: DocumentId,
        fps_counter: UpdateCounter,
        time: &mut Time,
    ) -> Result<(), UpdateError> {
        let _span = trace_span!("GameState::update").entered();

        while let Ok(event) = self.rx.try_recv() {
            match event {
                MenuEvent::Connect(addr) => {
                    if let Some(ctx) = self.root_ctx.take() {
                        ctx.remove_self();
                    }

                    self.connect(addr);
                }
                MenuEvent::Exit => return Err(UpdateError::Exit),
                MenuEvent::SpawnMainMenu => {
                    if let Some(ctx) = self.root_ctx.take() {
                        ctx.remove_self();
                    }

                    let ctx = ui_rt.root_context(doc);
                    self.root_ctx = Some(
                        TitleMenu {
                            events: self.tx.clone(),
                        }
                        .mount(&ctx),
                    );
                }
                MenuEvent::SpawnMultiPlayerMenu => {
                    if let Some(ctx) = self.root_ctx.take() {
                        ctx.remove_self();
                    }

                    let ctx = ui_rt.root_context(doc);
                    self.root_ctx = Some(
                        MultiPlayerMenu {
                            events: self.tx.clone(),
                        }
                        .mount(&ctx),
                    );
                }
            }
        }

        match &mut self.inner {
            GameStateInner::Startup => {
                self.inner = GameStateInner::MainMenu(MainMenuState::new(world));

                let ctx = ui_rt.root_context(doc);
                self.root_ctx = Some(
                    TitleMenu {
                        events: self.tx.clone(),
                    }
                    .mount(&ctx),
                );
            }
            GameStateInner::GameWorld(state) => {
                match state
                    .update(world, ui_rt, doc, fps_counter, &mut self.executor)
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

    pub fn handle_event(
        &mut self,
        event: WindowEvent,
        cursor: &Cursor,
        ui_rt: &Runtime,
        doc: DocumentId,
    ) {
        match &mut self.inner {
            GameStateInner::GameWorld(state) => {
                state.handle_event(event, cursor, ui_rt, doc);
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
