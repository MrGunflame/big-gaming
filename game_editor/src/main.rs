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
use game_ui::render::style::{Background, BorderRadius, Bounds, Size, SizeVec2, Style};
use game_ui::UiState;
use game_window::windows::WindowBuilder;
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

    let windows = state.window_manager.windows().clone();
    let mut deferred_windows = vec![];
    state.window_manager.run(move || {
        while let Ok(event) = rx.try_recv() {
            let doc =
                windows::spawn_window(state.state.clone(), state.ui_state.runtime.clone(), event);

            let id = windows.spawn(WindowBuilder::new());

            deferred_windows.push((id, doc));
        }

        let mut index = 0;
        while index < deferred_windows.len() {
            let id = deferred_windows[index].0;
            let doc = &deferred_windows[index].1;

            if let Some(window) = windows.state(id) {
                let size = window.inner_size();

                state.render_state.create(id, window);

                let cam = Camera {
                    transform: Transform::default(),
                    projection: Projection::default(),
                    target: RenderTarget::Window(id),
                };

                state.render_state.entities.push_camera(cam);
                state.ui_state.create(id, size);
                *state.ui_state.get_mut(id).unwrap() = doc.clone();

                deferred_windows.remove(index);
            } else {
                index += 1;
            }
        }

        state.render_state.render();
    });
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
