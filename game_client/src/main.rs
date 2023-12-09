mod config;
mod entities;
mod input;
mod net;
mod scene;
mod state;
mod ui;
mod utils;
mod world;

use std::sync::Arc;

use clap::Parser;
use config::Config;
use game_core::hierarchy::TransformHierarchy;
use game_core::time::Time;
use game_render::Renderer;
use game_scene::scene2::SceneGraph;
use game_scene::SceneSpawner;
use game_script::executor::ScriptExecutor;
use game_tasks::TaskPool;
use game_ui::UiState;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId};
use game_window::{WindowManager, WindowManagerContext};
use glam::UVec2;
use input::Inputs;
use scene::{SceneEntities, SceneState};
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

    let executor = ScriptExecutor::new(res.server, res.record_targets);
    let inputs = Inputs::from_file("inputs");

    if let Some(addr) = args.connect {
        state = GameState::GameWorld(GameWorldState::new(
            &config,
            addr,
            res.modules,
            &cursor,
            executor,
            inputs,
        ));
    }

    let renderer = Renderer::new();
    let ui_state = UiState::new(&renderer);

    let app = App {
        window_id,
        renderer,
        state,
        scene: SceneState {
            spawner: SceneSpawner::default(),
            graph: SceneGraph::new(),
        },
        time: Time::new(),
        cursor,
        pool: TaskPool::new(8),
        hierarchy: TransformHierarchy::default(),
        ui_state,
        entities: SceneEntities::default(),
    };

    wm.run(app);
}

pub struct App {
    state: GameState,
    /// Primary window
    window_id: WindowId,
    renderer: Renderer,
    scene: SceneState,
    time: Time,
    cursor: Arc<Cursor>,
    pool: TaskPool,
    hierarchy: TransformHierarchy,
    ui_state: UiState,
    entities: SceneEntities,
}

impl game_window::App for App {
    fn update(&mut self, mut ctx: WindowManagerContext<'_>) {
        self.time.update();

        let window = ctx.windows.state(self.window_id).unwrap();

        match &mut self.state {
            GameState::Startup => {
                self.state = GameState::MainMenu(MainMenuState::new(
                    &mut self.scene,
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
                    &mut self.scene,
                    window,
                    &self.time,
                    &mut self.hierarchy,
                    &mut self.ui_state,
                );
            }
            _ => todo!(),
        }

        self.scene
            .spawner
            .update(&mut self.scene.graph, &self.pool, Some(&mut self.renderer));
        self.scene.graph.compute_transform();
        self.entities
            .update(&mut self.scene.graph, &mut self.renderer);
        self.scene.graph.clear_trackers();

        self.renderer.render(&self.pool);
        self.ui_state.run(&self.renderer, &mut ctx.windows);
    }

    fn handle_event(&mut self, ctx: WindowManagerContext<'_>, event: WindowEvent) {
        match event.clone() {
            WindowEvent::WindowCreated(event) => {
                debug_assert_eq!(event.window, self.window_id);

                let window = ctx.windows.state(event.window).unwrap();
                self.ui_state.create(event.window, window.inner_size());
                self.renderer.create(event.window, window);

                let cx = self.ui_state.get_mut(event.window).unwrap().root_scope();
            }
            WindowEvent::WindowResized(event) => {
                debug_assert_eq!(event.window, self.window_id);

                self.renderer
                    .resize(event.window, UVec2::new(event.width, event.height));
                self.ui_state
                    .resize(event.window, UVec2::new(event.width, event.height));
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
            WindowEvent::CursorMoved(event) => {}
            WindowEvent::CursorEntered(event) => {}
            WindowEvent::CursorLeft(event) => {}
            WindowEvent::WindowCloseRequested(event) => {}
            WindowEvent::KeyboardInput(event) => {}
            WindowEvent::MouseWheel(event) => {}
            WindowEvent::MouseButtonInput(event) => {}
            WindowEvent::MouseMotion(event) => {}
        }

        match &mut self.state {
            GameState::GameWorld(state) => state.handle_event(
                event.clone(),
                &self.cursor,
                &mut self.ui_state,
                self.window_id,
            ),
            _ => (),
        }

        self.ui_state.send_event(&self.cursor, event);
    }
}
