mod backend;
mod scene;
mod state;
mod widgets;
mod windows;
mod world;

use std::collections::HashMap;
use std::sync::{mpsc, Arc};

use backend::{Backend, Handle, Response};

use game_common::world::World;
use game_gizmos::Gizmos;
use game_render::camera::RenderTarget;
use game_render::options::MainPassOptions;
use game_render::{FpsLimit, Renderer};
use game_tasks::TaskPool;
use game_ui::{UiState, WindowProperties};
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId};
use game_window::{WindowManager, WindowManagerContext};
use glam::UVec2;
use scene::SceneEntities;
use state::module::Modules;
use state::record::Records;
use state::EditorState;
use tokio::runtime::Runtime;

use crate::windows::SpawnWindow;

struct State {
    window_manager: WindowManager,
    render_state: Renderer,
    ui_state: UiState,
    state: EditorState,
}

impl State {
    fn new(handle: Handle) -> (Self, mpsc::Receiver<SpawnWindow>) {
        let mut render_state = Renderer::new().unwrap();
        render_state.set_options(MainPassOptions {
            shading: game_render::options::ShadingMode::Albedo,
        });
        render_state.set_fps_limit(FpsLimit::limited(60.try_into().unwrap()));

        let (tx, rx) = mpsc::channel();

        let records = Records::new();
        let modules = game_core::modules::load_modules("mods").unwrap();
        for module in modules.iter() {
            for record in module.records.iter() {
                records.insert(module.id, record.clone());
            }
        }

        let state = EditorState {
            modules: Modules::default(),
            records,
            spawn_windows: tx,
            handle,
        };

        (
            Self {
                state,
                window_manager: WindowManager::new(),
                ui_state: UiState::new(&mut render_state),
                render_state: render_state,
            },
            rx,
        )
    }
}

fn main() {
    game_core::logger::init();

    let (backend, handle) = Backend::new();

    let (mut state, rx) = State::new(handle);

    let mut editor_state = state.state.clone();
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(backend.run(&mut editor_state));
    });

    let modules = game_core::modules::load_modules("mods").unwrap();

    state
        .state
        .spawn_windows
        .send(SpawnWindow::MainWindow)
        .unwrap();

    let gizmos = Gizmos::new(&mut state.render_state);

    let app = App {
        renderer: state.render_state,
        ui_state: state.ui_state,
        state: state.state,
        rx,
        cursor: state.window_manager.cursor().clone(),
        loading_windows: HashMap::new(),
        active_windows: HashMap::new(),
        scene: SceneEntities::default(),
        pool: TaskPool::new(8),
        gizmos,
        world: World::new(),
        modules,
    };

    state.window_manager.run(app);
}

fn load_from_backend(state: &mut EditorState, resp: Response) {
    match resp {
        Response::LoadModule(res) => match res {
            Ok((module, recs)) => {
                for (_, rec) in recs.iter() {
                    state.records.insert(module.module.id, rec.clone());
                }

                state.modules.insert(module.clone());
            }
            Err(err) => {
                tracing::error!("failed to load module: {}", err);

                let msg = format!("failed to load module: {}", err);

                let _ = state.spawn_windows.send(SpawnWindow::Error(msg));
            }
        },
        Response::WriteModule(res) => match res {
            Ok(()) => {}
            Err(err) => {
                let _ = state.spawn_windows.send(SpawnWindow::Error(format!(
                    "failed to save modules: {}",
                    err
                )));
            }
        },
    }
}

pub struct App {
    renderer: Renderer,
    ui_state: UiState,
    rx: mpsc::Receiver<SpawnWindow>,
    state: EditorState,
    cursor: Arc<Cursor>,
    loading_windows: HashMap<WindowId, SpawnWindow>,
    active_windows: HashMap<WindowId, crate::windows::Window>,
    scene: SceneEntities,
    pool: TaskPool,
    gizmos: Gizmos,
    world: World,
    modules: game_core::modules::Modules,
}

impl game_window::App for App {
    fn handle_event(&mut self, ctx: WindowManagerContext<'_>, event: WindowEvent) {
        match event.clone() {
            WindowEvent::WindowCreated(event) => {
                let window = ctx.windows.state(event.window).unwrap();
                let size = window.inner_size();
                let scale_factor = window.scale_factor();
                dbg!(scale_factor);

                self.renderer.create(event.window, window);
                self.ui_state.create(
                    RenderTarget::Window(event.window),
                    WindowProperties { size, scale_factor },
                );

                if let Some(spawn) = self.loading_windows.remove(&event.window) {
                    let window = crate::windows::spawn_window(
                        &mut self.world,
                        &mut self.renderer,
                        self.state.clone(),
                        &self.ui_state,
                        spawn,
                        event.window,
                        self.modules.clone(),
                    );

                    self.active_windows.insert(event.window, window);
                }
            }
            WindowEvent::WindowResized(event) => {
                self.renderer
                    .resize(event.window, UVec2::new(event.width, event.height));
                self.ui_state.resize(
                    RenderTarget::Window(event.window),
                    UVec2::new(event.width, event.height),
                );
            }
            WindowEvent::WindowDestroyed(event) => {
                self.renderer.destroy(event.window);
                self.ui_state.destroy(RenderTarget::Window(event.window));

                self.active_windows.remove(&event.window);
            }
            WindowEvent::WindowCloseRequested(event) => {
                // TODO: Ask for confirmation if the window contains
                // unsaved data.
                ctx.windows.despawn(event.window);
            }
            WindowEvent::WindowScaleFactorChanged(event) => {
                self.ui_state
                    .update_scale_factor(RenderTarget::Window(event.window), event.scale_factor);
            }
            WindowEvent::MouseMotion(event) => {
                if let Some(window_id) = self.cursor.window() {
                    if let Some(window) = self.active_windows.get_mut(&window_id) {
                        window.handle_event(
                            &mut self.renderer,
                            WindowEvent::MouseMotion(event),
                            window_id,
                        );
                    }
                }
            }
            WindowEvent::KeyboardInput(event) => {
                if let Some(window_id) = self.cursor.window() {
                    if let Some(window) = self.active_windows.get_mut(&window_id) {
                        window.handle_event(
                            &mut self.renderer,
                            WindowEvent::KeyboardInput(event),
                            window_id,
                        );
                    }
                }
            }
            WindowEvent::MouseWheel(event) => {
                if let Some(window_id) = self.cursor.window() {
                    if let Some(window) = self.active_windows.get_mut(&window_id) {
                        window.handle_event(
                            &mut self.renderer,
                            WindowEvent::MouseWheel(event),
                            window_id,
                        );
                    }
                }
            }
            WindowEvent::MouseButtonInput(event) => {
                if let Some(window_id) = self.cursor.window() {
                    if let Some(window) = self.active_windows.get_mut(&window_id) {
                        window.handle_event(
                            &mut self.renderer,
                            WindowEvent::MouseButtonInput(event),
                            window_id,
                        );
                    }
                }
            }
            WindowEvent::CursorMoved(event) => {
                if let Some(window_id) = self.cursor.window() {
                    if let Some(window) = self.active_windows.get_mut(&window_id) {
                        window.handle_event(
                            &mut self.renderer,
                            WindowEvent::CursorMoved(event),
                            window_id,
                        );
                    }
                }
            }
            _ => (),
        }

        self.ui_state.send_event(&self.cursor, event);
    }

    fn update(&mut self, mut ctx: WindowManagerContext<'_>) {
        while let Ok(event) = self.rx.try_recv() {
            let id = ctx.windows.spawn(WindowBuilder::new());

            self.loading_windows.insert(id, event);
        }

        for (id, window) in self.active_windows.iter_mut() {
            window.update(&mut self.world, &mut self.renderer);

            // if matches!(window, crate::windows::Window::View(_, _)) {
            self.scene.update(
                &self.state.records,
                &self.world,
                &self.pool,
                &mut self.renderer,
                *id,
                &self.gizmos,
            );
            // }
        }

        self.ui_state.update();
        self.gizmos.swap_buffers();

        self.renderer.render(&self.pool);
    }
}
