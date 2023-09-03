#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod config;
//mod entities;
mod net;
//mod plugins;
mod state;
mod utils;

use clap::Parser;
use config::Config;
use game_core::counter::Interval;
use game_core::logger::{self};
use game_render::RenderState;
use game_scene::Scenes;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId, Windows};
use game_window::WindowManager;
use glam::UVec2;
use state::main_menu::MainMenuState;
use state::GameState;

use crate::net::ServerConnection;

#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    connect: Option<String>,
}

fn main() {
    logger::init();

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

    let mut app = App {
        window_id,
        conn: ServerConnection::new(&config),
        renderer: RenderState::new(),
        windows: wm.windows().clone(),
        state: GameState::Startup,
        scenes: Scenes::new(),
    };

    wm.run(app);
}

pub struct App {
    state: GameState,
    /// Primary window
    window_id: WindowId,
    conn: ServerConnection<Interval>,
    renderer: RenderState,
    windows: Windows,
    scenes: Scenes,
}

impl game_window::App for App {
    fn update(&mut self) {
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
            _ => todo!(),
        }

        game_scene::tick(&mut self.scenes, &mut self.renderer);
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
    }
}
