mod components;
mod config;
mod entities;
mod input;
mod net;
mod scene;
mod state;
mod ui;
mod utils;
mod world;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use clap::Parser;
use config::Config;
use game_common::sync::spsc;
use game_common::world::World;
use game_core::counter::{Interval, UpdateCounter};
use game_core::time::Time;
use game_render::camera::RenderTarget;
use game_render::Renderer;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use game_ui::events::WindowCommand;
use game_ui::reactive::Document;
use game_ui::UiState;
use game_window::cursor::Cursor;
use game_window::events::{WindowCloseRequested, WindowEvent};
use game_window::windows::{WindowBuilder, WindowId};
use game_window::{WindowManager, WindowManagerContext};
use glam::UVec2;
use input::Inputs;
use parking_lot::Mutex;
use scene::SceneEntities;
use state::main_menu::MainMenuState;
use state::GameState;
use world::GameWorldState;

#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    connect: Option<String>,
}

fn main() {
    game_tracing::init();

    let args = Args::parse();

    let mut config_path = std::env::current_dir().unwrap();
    config_path.push("config.toml");
    let config = match Config::from_file(&config_path) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("failed to load config file from {:?}: {}", config_path, err);
            return;
        }
    };

    let res = game_core::modules::load_modules();

    let mut wm = WindowManager::new();
    let window_id = wm.windows_mut().spawn(WindowBuilder::new());

    let mut state = GameState::Startup;

    let cursor = wm.cursor().clone();

    let inputs = Inputs::from_file("inputs");

    if let Some(addr) = args.connect {
        state = GameState::GameWorld(GameWorldState::new(
            &config,
            addr,
            res.modules,
            &cursor,
            res.executor,
            inputs,
        ));
    }

    let renderer = Renderer::new();
    let ui_state = UiState::new(&renderer);
    let events = spsc::Queue::new(8192);
    let (events_tx, events_rx) = events.split();

    // Lazy initialize the main window document for the game thread. We cannot
    // create the document before the main window is created, which does not
    // happen until we give control of the main thread to the windowing loop.
    let ui_doc = OnceLock::new();

    let pool = TaskPool::new(8);
    let world = Mutex::new(World::new());
    let fps_counter = Mutex::new(UpdateCounter::new());
    let shutdown = AtomicBool::new(false);
    let window_commands = Mutex::new(Vec::new());

    let game_state = GameAppState {
        state,
        world: &world,
        time: Time::new(),
        ui_doc: &ui_doc,
        events: events_rx,
        cursor: cursor.clone(),
        fps_counter: &fps_counter,
        shutdown: &shutdown,
        interval: Interval::new(Duration::from_secs(1) / 60),
        ui_state,
        window_commands: &window_commands,
        pool: &pool,
    };

    let renderer_state = RendererAppState {
        renderer,
        entities: SceneEntities::default(),
        world: &world,
        pool: &pool,
        window_id,
        ui_doc: &ui_doc,
        cursor,
        events: events_tx,
        fps_counter: &fps_counter,
        shutdown: &shutdown,
        window_commands: &window_commands,
    };

    std::thread::scope(|scope| {
        scope.spawn(|| {
            game_state.run();
        });

        wm.run(renderer_state);
    });
}

pub struct GameAppState<'a> {
    state: GameState,
    world: &'a Mutex<World>,
    time: Time,
    ui_doc: &'a OnceLock<Document>,
    events: spsc::Receiver<WindowEvent>,
    cursor: Arc<Cursor>,
    fps_counter: &'a Mutex<UpdateCounter>,
    shutdown: &'a AtomicBool,
    interval: Interval,
    ui_state: UiState,
    window_commands: &'a Mutex<Vec<WindowCommand>>,
    pool: &'a TaskPool,
}

impl<'a> GameAppState<'a> {
    pub fn run(mut self) {
        while !self.shutdown.load(Ordering::Relaxed) {
            self.time.update();

            self.update();
        }
    }

    pub fn update(&mut self) {
        let _span = trace_span!("GameAppState::update").entered();

        let mut world = { self.world.lock().clone() };

        // If the renderer runs faster than the update we may have the same
        // event multiple times, but we only want't to handle it once per
        // update.
        let mut events: Vec<_> = self.events.drain().take(8192).collect();
        events.dedup();
        for event in events {
            // Handle window events for the UI.
            match event {
                WindowEvent::WindowCreated(event) => {
                    self.ui_state
                        .create(RenderTarget::Window(event.window), UVec2::ZERO);

                    let doc = self
                        .ui_state
                        .get_mut(RenderTarget::Window(event.window))
                        .unwrap()
                        .clone();
                    let _ = self.ui_doc.set(doc);
                    continue;
                }
                WindowEvent::WindowResized(event) => {
                    self.ui_state
                        .resize(RenderTarget::Window(event.window), event.size());
                    continue;
                }
                WindowEvent::WindowDestroyed(event) => {
                    self.ui_state.destroy(RenderTarget::Window(event.window));
                    continue;
                }
                _ => (),
            }

            self.ui_state.send_event(&self.cursor, event.clone());

            if let Some(ui_doc) = self.ui_doc.get() {
                match &mut self.state {
                    GameState::GameWorld(state) => {
                        state.handle_event(event, &self.cursor, ui_doc);
                    }
                    _ => (),
                }
            }
        }

        let fps_counter = { self.fps_counter.lock().clone() };

        if let Some(ui_doc) = self.ui_doc.get() {
            match &mut self.state {
                GameState::Startup => {
                    self.state = GameState::MainMenu(MainMenuState::new(&mut world))
                }
                GameState::MainMenu(state) => {
                    state.update(&mut world);
                }
                GameState::GameWorld(state) => {
                    state.update(&self.time, &mut world, ui_doc, fps_counter)
                }
                _ => todo!(),
            }
        }

        *self.world.lock() = world;

        self.ui_state.update(&mut self.window_commands.lock());
    }
}

pub struct RendererAppState<'a> {
    renderer: Renderer,
    entities: SceneEntities,
    world: &'a Mutex<World>,
    pool: &'a TaskPool,
    window_id: WindowId,
    ui_doc: &'a OnceLock<Document>,
    cursor: Arc<Cursor>,
    events: spsc::Sender<WindowEvent>,
    fps_counter: &'a Mutex<UpdateCounter>,
    shutdown: &'a AtomicBool,
    window_commands: &'a Mutex<Vec<WindowCommand>>,
}

impl<'a> game_window::App for RendererAppState<'a> {
    fn update(&mut self, ctx: WindowManagerContext<'_>) {
        let _span = trace_span!("RendererAppState::update").entered();

        let cmds = { std::mem::take(&mut *self.window_commands.lock()) };
        for cmd in cmds {
            match cmd {
                WindowCommand::Close(id) => {
                    ctx.windows.despawn(id);
                }
                WindowCommand::SetCursorIcon(id, icon) => {
                    if let Some(state) = ctx.windows.state(id) {
                        state.set_cursor_icon(icon);
                    }
                }
                WindowCommand::SetTitle(id, title) => {
                    if let Some(state) = ctx.windows.state(id) {
                        state.set_title(&title);
                    }
                }
            }
        }

        // Wait until the last vsync is done before we start preparing the next
        // frame. This helps combat latency issues and will not cause stalls
        // when using multiple buffers.
        self.renderer.wait_until_ready();

        let world = { self.world.lock().clone() };

        self.entities
            .update(&world, &self.pool, &mut self.renderer, self.window_id);

        self.renderer.render(&self.pool);

        self.fps_counter.lock().update();
    }

    fn handle_event(&mut self, mut ctx: WindowManagerContext<'_>, event: WindowEvent) {
        match event.clone() {
            WindowEvent::WindowCreated(event) => {
                debug_assert_eq!(event.window, self.window_id);

                let window = ctx.windows.state(event.window).unwrap();

                self.renderer.create(event.window, window);
            }
            WindowEvent::WindowResized(event) => {
                debug_assert_eq!(event.window, self.window_id);

                self.renderer
                    .resize(event.window, UVec2::new(event.width, event.height));
            }
            WindowEvent::WindowDestroyed(event) => {
                // Note that this can only be the primary window as
                // we never spawn another window.
                debug_assert_eq!(event.window, self.window_id);

                self.renderer.destroy(event.window);

                self.shutdown.store(true, Ordering::Relaxed);
                ctx.exit();
            }
            WindowEvent::WindowCloseRequested(event) => {
                debug_assert_eq!(event.window, self.window_id);
                ctx.windows.despawn(event.window);
            }
            _ => (),
        }

        if let Err(_) = self.events.push(event.clone()) {
            tracing::error!("cannot send input event, queue is full");
        }
    }
}
