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

use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use config::{Config, ConfigError};
use game_common::sync::spsc;
use game_common::world::World;
use game_core::counter::{Interval, UpdateCounter};
use game_core::modules::{load_scripts, Modules};
use game_core::time::Time;
use game_crash_handler::main;
use game_gizmos::Gizmos;
use game_render::camera::RenderTarget;
use game_render::{FpsLimit, Renderer};
use game_script::Executor;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use game_ui::reactive::DocumentId;
use game_ui::UiState;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId};
use game_window::{WindowManager, WindowManagerContext};
use glam::UVec2;
use input::Inputs;
use parking_lot::Mutex;
use scene::SceneEntities;
use state::{GameState, UpdateError};

#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the config file.
    #[arg(short, long, value_name = "FILE", default_value = "config.toml")]
    config: PathBuf,
    /// Create a new config file. Ignored if config file already exists.
    #[arg(long)]
    create_config: bool,
    /// Connect to the server with the address after startup.
    #[arg(long)]
    connect: Option<String>,
    /// Path to the directory containing module archives.
    #[arg(short, long, value_name = "DIR", default_value = "mods")]
    mods: PathBuf,
    /// Disable the crash handler shim.
    // Note: This flag is handled by the crash handler shim
    // and should not be used by us.
    // It only exists so that the flag is included in the help
    // message.
    #[arg(long = "no-crash-handler")]
    _no_crash_handler: bool,
}

#[main]
fn main() -> ExitCode {
    game_core::logger::init();

    let args = Args::parse();

    let config = match Config::from_file(&args.config) {
        Ok(config) => config,
        Err(err) => match err {
            ConfigError::Io(err) if err.kind() == ErrorKind::NotFound && args.create_config => {
                tracing::info!(
                    "creating new config file at {}",
                    args.config.to_string_lossy(),
                );

                match Config::create_default_config(&args.config) {
                    Ok(config) => config,
                    Err(err) => {
                        tracing::error!(
                            "failed to create config file at {}: {}",
                            args.config.to_string_lossy(),
                            err,
                        );
                        return ExitCode::FAILURE;
                    }
                }
            }
            _ => {
                tracing::error!(
                    "failed to load config file from {}: {}",
                    args.config.to_string_lossy(),
                    err,
                );
                return ExitCode::FAILURE;
            }
        },
    };

    let modules = game_core::modules::load_modules(&args.mods).unwrap();

    let mut executor = Executor::new();
    load_scripts(&mut executor, &modules);

    let mut wm = WindowManager::new();
    let window_id = wm.windows_mut().spawn(WindowBuilder::new());

    let cursor = wm.cursor().clone();

    let inputs = Inputs::from_file("inputs");

    let mut state = GameState::new(
        config.clone(),
        modules.clone(),
        inputs,
        executor,
        cursor.clone(),
    );

    if let Some(addr) = args.connect {
        state.connect(addr);
    }

    let mut renderer = match Renderer::new() {
        Ok(renderer) => renderer,
        Err(err) => {
            tracing::error!("cannot create renderer: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if let Some(fps_limit) = config.graphics.fps_limit() {
        renderer.set_fps_limit(FpsLimit::limited(fps_limit));
    }

    let ui_state = UiState::new(&renderer);
    let gizmos = Gizmos::new(&renderer);
    let events = spsc::Queue::new(8192);
    let (events_tx, events_rx) = events.split();

    let pool = TaskPool::new(8);
    let world = Mutex::new(World::new());
    let fps_counter = Mutex::new(UpdateCounter::new());
    let shutdown = AtomicBool::new(false);

    let game_state = GameAppState {
        state,
        world: &world,
        time: Time::new(),
        events: events_rx,
        cursor: cursor.clone(),
        fps_counter: &fps_counter,
        shutdown: &shutdown,
        interval: Interval::new(Duration::from_secs(1) / 60),
        ui_state,
        pool: &pool,
        gizmos: &gizmos,
        document_id: None,
    };

    let renderer_state = RendererAppState {
        renderer,
        entities: SceneEntities::default(),
        world: &world,
        pool: &pool,
        window_id,
        events: events_tx,
        fps_counter: &fps_counter,
        shutdown: &shutdown,
        gizmos: &gizmos,
        modules: &modules,
    };

    std::thread::scope(|scope| {
        scope.spawn(|| {
            game_state.run();
        });

        wm.run(renderer_state);
    });

    ExitCode::SUCCESS
}

pub struct GameAppState<'a> {
    state: GameState,
    world: &'a Mutex<World>,
    time: Time,
    events: spsc::Receiver<WindowEvent>,
    cursor: Arc<Cursor>,
    fps_counter: &'a Mutex<UpdateCounter>,
    shutdown: &'a AtomicBool,
    interval: Interval,
    ui_state: UiState,
    gizmos: &'a Gizmos,
    pool: &'a TaskPool,
    document_id: Option<DocumentId>,
}

impl<'a> GameAppState<'a> {
    pub fn run(mut self) {
        while !self.shutdown.load(Ordering::Acquire) {
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
                    self.document_id = self
                        .ui_state
                        .runtime()
                        .create_document(RenderTarget::Window(event.window));

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

            if let Some(doc) = self.document_id {
                self.state
                    .handle_event(event, &self.cursor, &self.ui_state.runtime(), doc);
            }
        }

        let fps_counter = { self.fps_counter.lock().clone() };

        if let Some(doc) = self.document_id {
            match self.pool.block_on(self.state.update(
                &mut world,
                &self.ui_state.runtime(),
                doc,
                fps_counter,
                &mut self.time,
            )) {
                Ok(()) => (),
                Err(UpdateError::Exit) => {
                    self.shutdown.store(true, Ordering::Release);
                    return;
                }
            }
        }

        *self.world.lock() = world;

        self.ui_state.update();
        self.gizmos.swap_buffers();
    }
}

pub struct RendererAppState<'a> {
    renderer: Renderer,
    entities: SceneEntities,
    world: &'a Mutex<World>,
    pool: &'a TaskPool,
    window_id: WindowId,
    events: spsc::Sender<WindowEvent>,
    fps_counter: &'a Mutex<UpdateCounter>,
    shutdown: &'a AtomicBool,
    gizmos: &'a Gizmos,
    modules: &'a Modules,
}

impl<'a> game_window::App for RendererAppState<'a> {
    fn update(&mut self, mut ctx: WindowManagerContext<'_>) {
        let _span = trace_span!("RendererAppState::update").entered();

        if self.shutdown.load(Ordering::Acquire) {
            ctx.exit();
            return;
        }

        // Wait until the last vsync is done before we start preparing the next
        // frame. This helps combat latency issues and will not cause stalls
        // when using multiple buffers.
        self.renderer.wait_until_ready();

        let world = { self.world.lock().clone() };

        self.entities.update(
            &self.modules,
            &world,
            &self.pool,
            &mut self.renderer,
            self.window_id,
            &self.gizmos,
        );

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

                self.shutdown.store(true, Ordering::Release);
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
