#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod backend;
mod state;
mod widgets;
mod windows;
mod world;

use std::sync::mpsc;

use backend::{Backend, Handle, Response};

use game_render::RenderState;
use game_ui::render::style::{Background, BorderRadius, Bounds, Size, SizeVec2, Style};
use game_ui::UiState;
use game_window::windows::WindowBuilder;
use game_window::WindowManager;
use glam::UVec2;
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
                render_state,
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

    create_main_window(&mut state);

    {
        let windows = state.window_manager.windows().clone();
        std::thread::spawn(move || {
            let mut deferred_windows = vec![];

            while let Ok(event) = rx.try_recv() {
                let doc = windows::spawn_window(
                    state.state.clone(),
                    state.ui_state.runtime.clone(),
                    event,
                );

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
                    state.ui_state.create(id, size);
                    *state.ui_state.get_mut(id).unwrap() = doc.clone();

                    deferred_windows.remove(index);
                } else {
                    index += 1;
                }
            }
        });
    }

    state.window_manager.run();
}

fn create_main_window(state: &mut State) {
    let id = state.window_manager.windows().spawn(WindowBuilder::new());

    state.ui_state.create(id, UVec2::ZERO);
    let document = state.ui_state.get_mut(id).unwrap();

    let buttons = vec![
        ActionButton {
            label: "Modules".to_owned(),
            on_click: {
                let state = state.state.clone();

                Box::new(move |_| {
                    let _ = state.spawn_windows.send(SpawnWindow::Modules);
                })
            },
        },
        ActionButton {
            label: "Records".to_owned(),
            on_click: {
                let state = state.state.clone();

                Box::new(move |_| {
                    let _ = state.spawn_windows.send(SpawnWindow::Records);
                })
            },
        },
        ActionButton {
            label: "View".to_owned(),
            on_click: {
                let state = state.state.clone();

                Box::new(move |_| {
                    let _ = state.spawn_windows.send(SpawnWindow::View);
                })
            },
        },
    ];

    let cx = document.root_scope();
    game_ui::view! {
        cx,
        <ToolBar buttons={buttons}>
        </ToolBar>
    };

    let style = Style {
        background: Background::AQUA,
        bounds: Bounds {
            min: SizeVec2::splat(Size::Pixels(64.0)),
            max: SizeVec2::splat(Size::Pixels(64.0)),
        },
        border_radius: BorderRadius::splat(Size::Pixels(16.0)),
        ..Default::default()
    };

    game_ui::view! {cx,
        <Container style={style}>
        </Container>
    };
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
