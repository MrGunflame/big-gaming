#![deny(unsafe_op_in_unsafe_fn)]

mod backend;
mod state;
mod widgets;
mod windows;
mod world;

use std::collections::HashMap;
use std::sync::{mpsc, Arc};

use backend::{Backend, Handle, Response};

use game_render::Renderer;
use game_scene::Scenes;
use game_ui::UiState;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId, Windows};
use game_window::WindowManager;
use glam::UVec2;
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
        let mut render_state = Renderer::new();

        let (tx, rx) = mpsc::channel();

        let state = EditorState {
            modules: Modules::default(),
            records: Records::default(),
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
    game_tracing::init();

    let (backend, handle) = Backend::new();

    let (mut state, rx) = State::new(handle);

    let mut editor_state = state.state.clone();
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(backend.run(&mut editor_state));
    });

    state
        .state
        .spawn_windows
        .send(SpawnWindow::MainWindow)
        .unwrap();

    let app = App {
        renderer: state.render_state,
        ui_state: state.ui_state,
        state: state.state,
        rx,
        windows: state.window_manager.windows().clone(),
        cursor: state.window_manager.cursor().clone(),
        loading_windows: HashMap::new(),
        active_windows: HashMap::new(),
        scenes: Scenes::new(),
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
    windows: Windows,
    rx: mpsc::Receiver<SpawnWindow>,
    state: EditorState,
    cursor: Arc<Cursor>,
    loading_windows: HashMap<WindowId, SpawnWindow>,
    active_windows: HashMap<WindowId, crate::windows::Window>,
    scenes: Scenes,
}

impl game_window::App for App {
    fn handle_event(&mut self, event: game_window::events::WindowEvent) {
        match event {
            WindowEvent::WindowCreated(event) => {
                let window = self.windows.state(event.window).unwrap();
                let size = window.inner_size();

                self.renderer.create(event.window, window);
                self.ui_state.create(event.window, size);

                if let Some(spawn) = self.loading_windows.remove(&event.window) {
                    let window = crate::windows::spawn_window(
                        &mut self.renderer,
                        &mut self.scenes,
                        self.state.clone(),
                        self.ui_state.runtime.clone(),
                        spawn,
                        event.window,
                    );

                    if let Some(doc) = window.doc() {
                        *self.ui_state.get_mut(event.window).unwrap() = doc;
                    }

                    self.active_windows.insert(event.window, window);
                }
            }
            WindowEvent::WindowResized(event) => {
                self.renderer
                    .resize(event.window, UVec2::new(event.width, event.height));
                self.ui_state
                    .resize(event.window, UVec2::new(event.width, event.height));
            }
            WindowEvent::WindowDestroyed(event) => {
                self.renderer.destroy(event.window);
                self.ui_state.destroy(event.window);

                self.active_windows.remove(&event.window);
            }
            WindowEvent::WindowCloseRequested(event) => {
                // TODO: Ask for confirmation if the window contains
                // unsaved data.
                self.windows.despawn(event.window);
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

    fn update(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            let id = self.windows.spawn(WindowBuilder::new());

            self.loading_windows.insert(id, event);
        }

        self.scenes.update(&mut self.renderer);

        self.renderer.render();
        self.ui_state
            .run(&self.renderer.device, &self.renderer.queue, &self.windows);
    }
}
