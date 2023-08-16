#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod backend;
mod state;
mod widgets;
mod windows;
mod world;

use std::sync::{mpsc, Arc};

use backend::{Backend, Handle, Response};

use game_common::components::transform::Transform;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::RenderState;
use game_ui::reactive::Document;
use game_ui::render::style::{Background, BorderRadius, Bounds, Size, SizeVec2, Style};
use game_ui::UiState;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId, Windows};
use game_window::WindowManager;
use glam::UVec2;
use parking_lot::Mutex;
use state::module::Modules;
use state::record::Records;
use state::EditorState;
use tokio::runtime::Runtime;
use widgets::tool_bar::ToolBar;

use game_ui::widgets::*;
use widgets::tool_bar::*;

use crate::windows::SpawnWindow;

struct State {
    window_manager: WindowManager,
    render_state: RenderState,
    ui_state: UiState,
    state: EditorState,
}

impl State {
    fn new(handle: Handle) -> (Self, mpsc::Receiver<SpawnWindow>) {
        let mut render_state = RenderState::new();

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
    pretty_env_logger::init();

    let (backend, handle) = Backend::new();

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(backend.run());
    });

    let (mut state, rx) = State::new(handle);

    state
        .state
        .spawn_windows
        .send(SpawnWindow::MainWindow)
        .unwrap();

    let app = App {
        renderer: state.render_state,
        ui_state: state.ui_state,
        deferred_windows: vec![],
        state: state.state,
        rx,
        windows: state.window_manager.windows().clone(),
        cursor: state.window_manager.cursor().clone(),
    };

    state.window_manager.run(app);
}

fn load_from_backend(state: EditorState) {
    while let Some(resp) = state.handle.recv() {
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
}

pub struct App {
    renderer: RenderState,
    ui_state: UiState,
    deferred_windows: Vec<(WindowId, Document)>,
    windows: Windows,
    rx: mpsc::Receiver<SpawnWindow>,
    state: EditorState,
    cursor: Arc<Cursor>,
}

impl game_window::App for App {
    fn handle_event(&mut self, event: game_window::events::WindowEvent) {
        match event {
            WindowEvent::WindowResized(event) => {
                self.renderer
                    .resize(event.window, UVec2::new(event.width, event.height));
                self.ui_state
                    .resize(event.window, UVec2::new(event.width, event.height));
            }
            _ => (),
        }

        self.ui_state.send_event(&self.cursor, event);
    }

    fn update(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            let doc =
                windows::spawn_window(self.state.clone(), self.ui_state.runtime.clone(), event);

            let id = self.windows.spawn(WindowBuilder::new());

            self.deferred_windows.push((id, doc));
        }

        let mut index = 0;
        while index < self.deferred_windows.len() {
            let id = self.deferred_windows[index].0;
            let doc = &self.deferred_windows[index].1;

            if let Some(window) = self.windows.state(id) {
                let size = window.inner_size();

                self.renderer.create(id, window);

                let cam = Camera {
                    transform: Transform::default(),
                    projection: Projection::default(),
                    target: RenderTarget::Window(id),
                };

                self.renderer.entities.push_camera(cam);
                self.ui_state.create(id, size);
                *self.ui_state.get_mut(id).unwrap() = doc.clone();

                self.deferred_windows.remove(index);
            } else {
                index += 1;
            }
        }

        self.renderer.render();
        self.ui_state
            .run(&self.renderer.device, &self.renderer.queue);
    }
}
