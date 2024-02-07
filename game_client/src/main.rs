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

use std::sync::{Arc, Mutex, OnceLock};

use clap::Parser;
use config::Config;
use game_common::world::World;
use game_core::counter::UpdateCounter;
use game_core::time::Time;
use game_render::Renderer;
use game_tasks::TaskPool;
use game_ui::reactive::Document;
use game_ui::UiState;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId};
use game_window::{WindowManager, WindowManagerContext};
use glam::UVec2;
use input::Inputs;
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
    let events = Mutex::new(Vec::new());

    // Lazy initialize the main window document for the game thread. We cannot
    // create the document before the main window is created, which does not
    // happen until we give control of the main thread to the windowing loop.
    let ui_doc = OnceLock::new();

    let pool = TaskPool::new(8);
    let world = Mutex::new(World::new());
    let fps_counter = Mutex::new(UpdateCounter::new());

    let game_state = GameAppState {
        state,
        world: &world,
        time: Time::new(),
        ui_doc: &ui_doc,
        events: &events,
        cursor: cursor.clone(),
        fps_counter: &fps_counter,
    };

    let renderer_state = RendererAppState {
        renderer,
        entities: SceneEntities::default(),
        world: &world,
        pool: &pool,
        ui_state,
        window_id,
        ui_doc: &ui_doc,
        cursor,
        events: &events,
        fps_counter: &fps_counter,
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
    events: &'a Mutex<Vec<WindowEvent>>,
    cursor: Arc<Cursor>,
    fps_counter: &'a Mutex<UpdateCounter>,
}

impl<'a> GameAppState<'a> {
    pub fn run(mut self) -> ! {
        loop {
            self.update();
        }
    }

    pub fn update(&mut self) {
        let Some(ui_doc) = self.ui_doc.get() else {
            return;
        };

        self.time.update();

        let mut world = self.world.lock().unwrap().clone();

        let events = std::mem::take(&mut *self.events.lock().unwrap());
        for event in events {
            match &mut self.state {
                GameState::GameWorld(state) => {
                    state.handle_event(event, &self.cursor, ui_doc);
                }
                _ => (),
            }
        }

        match &mut self.state {
            GameState::Startup => self.state = GameState::MainMenu(MainMenuState::new(&mut world)),
            GameState::MainMenu(state) => {
                state.update(&mut world);
            }
            GameState::GameWorld(state) => state.update(
                &self.time,
                &mut world,
                &ui_doc,
                self.fps_counter.lock().unwrap().clone(),
            ),
            _ => todo!(),
        }

        *self.world.lock().unwrap() = world;
    }
}

pub struct RendererAppState<'a> {
    renderer: Renderer,
    entities: SceneEntities,
    world: &'a Mutex<World>,
    pool: &'a TaskPool,
    ui_state: UiState,
    window_id: WindowId,
    ui_doc: &'a OnceLock<Document>,
    cursor: Arc<Cursor>,
    events: &'a Mutex<Vec<WindowEvent>>,
    fps_counter: &'a Mutex<UpdateCounter>,
}

impl<'a> game_window::App for RendererAppState<'a> {
    fn update(&mut self, mut ctx: WindowManagerContext<'_>) {
        let world = self.world.lock().unwrap();

        self.entities
            .update(&world, &self.pool, &mut self.renderer, self.window_id);

        self.renderer.render(&self.pool);
        self.ui_state.run(&self.renderer, &mut ctx.windows);

        self.fps_counter.lock().unwrap().update();
    }

    fn handle_event(&mut self, ctx: WindowManagerContext<'_>, event: WindowEvent) {
        match event.clone() {
            WindowEvent::WindowCreated(event) => {
                debug_assert_eq!(event.window, self.window_id);

                let window = ctx.windows.state(event.window).unwrap();
                self.ui_state.create(event.window, window.inner_size());

                if window.id() == self.window_id {
                    let doc = self.ui_state.get_mut(self.window_id).unwrap().clone();
                    let _ = self.ui_doc.set(doc);
                }

                self.renderer.create(event.window, window);
                return;
            }
            WindowEvent::WindowResized(event) => {
                debug_assert_eq!(event.window, self.window_id);

                self.renderer
                    .resize(event.window, UVec2::new(event.width, event.height));
                self.ui_state
                    .resize(event.window, UVec2::new(event.width, event.height));
                return;
            }
            WindowEvent::WindowDestroyed(event) => {
                // Note that this can only be the primary window as
                // we never spawn another window.
                debug_assert_eq!(event.window, self.window_id);

                self.renderer.destroy(event.window);
                self.ui_state.destroy(event.window);

                tracing::info!("primary window destroyed; exiting");
                std::process::exit(0);
            }
            _ => (),
        }

        self.events.lock().unwrap().push(event.clone());
        self.ui_state.send_event(&self.cursor, event);
    }
}
