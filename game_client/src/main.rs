#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod config;
mod entities;
mod input;
mod net;
mod state;
mod utils;
mod world;

use std::sync::Arc;

use clap::Parser;
use config::Config;
use game_core::hierarchy::TransformHierarchy;
use game_core::time::Time;
use game_render::Renderer;
use game_scene::Scenes;
use game_script::executor::ScriptExecutor;
use game_tasks::TaskPool;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId, Windows};
use game_window::WindowManager;
use glam::UVec2;
use input::Inputs;
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
    let window_id = wm.windows().spawn(WindowBuilder::new());

    let mut state = GameState::Startup;

    let cursor = wm.cursor().clone();

    let executor = Arc::new(ScriptExecutor::new(res.server, res.record_targets));
    let inputs = Inputs::from_file("inputs");

    if let Some(addr) = args.connect {
        state = GameState::GameWorld(GameWorldState::new(
            &config,
            addr,
            res.modules,
            &cursor,
            executor.clone(),
            inputs,
        ));
    }

    let app = App {
        window_id,
        renderer: Renderer::new(),
        windows: wm.windows().clone(),
        state,
        scenes: Scenes::new(),
        time: Time::new(),
        cursor: cursor,
        pool: TaskPool::new(8),
        hierarchy: TransformHierarchy::default(),
        executor,
    };

    wm.run(app);
}

pub struct App {
    state: GameState,
    /// Primary window
    window_id: WindowId,
    renderer: Renderer,
    windows: Windows,
    scenes: Scenes,
    time: Time,
    cursor: Arc<Cursor>,
    pool: TaskPool,
    hierarchy: TransformHierarchy,
    // TODO: No need for Arc here, but we want the executor in game state
    // not App state.
    executor: Arc<ScriptExecutor>,
}

impl game_window::App for App {
    fn update(&mut self) {
        self.time.update();

        let window = self.windows.state(self.window_id).unwrap();

        match &mut self.state {
            GameState::Startup => {
                self.state = GameState::MainMenu(MainMenuState::new(
                    &mut self.scenes,
                    &mut self.renderer,
                    self.window_id,
                    &mut self.hierarchy,
                ));
            }
            GameState::MainMenu(state) => {
                state.update(&mut self.renderer);
            }
            GameState::GameWorld(state) => {
                state.update(
                    &mut self.renderer,
                    &mut self.scenes,
                    window,
                    &self.time,
                    &mut self.hierarchy,
                );
            }
            _ => todo!(),
        }

        self.hierarchy.compute_transform();
        self.scenes
            .update(&mut self.hierarchy, &mut self.renderer, &self.pool);
        self.renderer.render(&self.pool);
    }

    fn handle_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::WindowCreated(event) => {
                debug_assert_eq!(event.window, self.window_id);

                let window = self.windows.state(event.window).unwrap();
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

                tracing::info!("primary window destroyed; exiting");
                std::process::exit(0);
            }
            WindowEvent::CursorMoved(event) => {}
            WindowEvent::CursorEntered(event) => {}
            WindowEvent::CursorLeft(event) => {}
            WindowEvent::ReceivedCharacter(event) => {}
            WindowEvent::WindowCloseRequested(event) => {}
            WindowEvent::KeyboardInput(event) => {}
            WindowEvent::MouseWheel(event) => {}
            WindowEvent::MouseButtonInput(event) => {}
            WindowEvent::MouseMotion(event) => {}
        }

        match &mut self.state {
            GameState::GameWorld(state) => state.handle_event(event, &self.cursor),
            _ => (),
        }
    }
}
