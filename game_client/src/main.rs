#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod config;
mod entities;
mod net;
mod state;
mod utils;
mod world;

use clap::Parser;
use config::Config;
use game_core::counter::Interval;
use game_core::logger::{self};
use game_core::time::Time;
use game_render::Renderer;
use game_scene::Scenes;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId, Windows};
use game_window::WindowManager;
use glam::UVec2;
use state::main_menu::MainMenuState;
use state::GameState;
use world::GameWorldState;

use crate::net::ServerConnection;

#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    connect: Option<String>,
}

fn main() {
    // logger::init();
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

    if let Some(addr) = args.connect {
        state = GameState::GameWorld(GameWorldState::new(&config, addr, res.modules));
    }

    let app = App {
        window_id,
        renderer: Renderer::new(),
        windows: wm.windows().clone(),
        state,
        scenes: Scenes::new(),
        time: Time::new(),
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
}

impl game_window::App for App {
    fn update(&mut self) {
        self.time.update();

        match &mut self.state {
            GameState::Startup => {
                self.state = GameState::MainMenu(MainMenuState::new(
                    &mut self.scenes,
                    &mut self.renderer,
                    self.window_id,
                ));
            }
            GameState::MainMenu(state) => {
                state.update(&mut self.renderer);
            }
            GameState::GameWorld(state) => {
                state.update(
                    &mut self.renderer,
                    &mut self.scenes,
                    self.window_id,
                    &self.time,
                );
            }
            _ => todo!(),
        }

        self.scenes.update(&mut self.renderer);
        self.renderer.render();
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
            GameState::GameWorld(state) => state.handle_event(&mut self.scenes, event),
            _ => (),
        }
    }
}
