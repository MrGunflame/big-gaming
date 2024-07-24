use std::sync::mpsc;

use game_common::world::World;
use game_core::counter::UpdateCounter;
use game_core::time::Time;
use game_tracing::trace_span;
use game_ui::reactive::{Context, DocumentId, Runtime};
use game_ui::widgets::Widget;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;

use crate::ui::title_menu::{MenuEvent, MultiPlayerMenu, TitleMenu};
use crate::world::GameWorldState;

use self::main_menu::MainMenuState;

pub mod main_menu;

pub struct GameState {
    inner: GameStateInner,
    tx: mpsc::Sender<MenuEvent>,
    rx: mpsc::Receiver<MenuEvent>,
    root_ctx: Option<Context<()>>,
}

impl GameState {
    pub fn new(inner: GameStateInner) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            inner,
            tx,
            rx,
            root_ctx: None,
        }
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
                    todo!()
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
                if let Err(err) = state.update(world, ui_rt, doc, fps_counter).await {
                    panic!("{:?}", err);
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
pub enum GameStateInner {
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
