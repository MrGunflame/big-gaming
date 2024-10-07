mod create_module;
mod edit_prefab;
mod edit_world;
mod error;
pub mod main_window;
pub mod modules;
mod open_module;
mod record;
mod records;
mod world;

use std::sync::Arc;

use create_module::EditModule;
use edit_prefab::EditPrefabWindow;
use edit_world::EditWorldWindow;
use game_common::world::World;
use game_data::record::RecordKind;
use game_render::camera::RenderTarget;
use game_render::options::MainPassOptions;
use game_render::Renderer;
use game_tracing::trace_span;
use game_ui::runtime::DocumentId;
use game_ui::widgets::{Callback, Widget};
use game_ui::UiState;
use game_wasm::record::ModuleId;
use game_wasm::world::RecordReference;
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use modules::Modules;
use parking_lot::Mutex;
use record::{EditRecord, EditState};
use records::Records;

use crate::state::EditorState;
use crate::widgets::explorer::{Entry, Explorer};
use crate::windows::create_module::CreateModule;
use crate::windows::error::Error;

use self::main_window::MainWindow;
use self::open_module::OpenModule;

trait WindowTrait {
    fn handle_event(&mut self, renderer: &mut Renderer, event: WindowEvent, window_id: WindowId);

    fn update(&mut self, world: &mut World, renderer: &mut Renderer, options: &mut MainPassOptions);
}

pub struct Window {
    document: DocumentId,
    inner: Option<Box<dyn WindowTrait>>,
}

impl Window {
    pub fn handle_event(
        &mut self,
        renderer: &mut Renderer,
        event: WindowEvent,
        window_id: WindowId,
    ) {
        let _span = trace_span!("Window::handle_event").entered();

        if let Some(inner) = &mut self.inner {
            inner.handle_event(renderer, event, window_id);
        }
    }

    pub fn update(&mut self, world: &mut World, renderer: &mut Renderer) {
        let _span = trace_span!("Window::update").entered();

        if let Some(inner) = &mut self.inner {
            let mut options = MainPassOptions::default();
            inner.update(world, renderer, &mut options);
            renderer.set_options(options);
        }
    }
}

pub fn spawn_window(
    world: &mut World,
    renderer: &mut Renderer,
    state: EditorState,
    ui_state: &UiState,
    event: SpawnWindow,
    window_id: WindowId,
    modules: game_core::modules::Modules,
) -> Window {
    let rt = ui_state.runtime();
    if rt.documents(RenderTarget::Window(window_id)).is_empty() {
        rt.create_document(RenderTarget::Window(window_id));
    }
    let document = rt.documents(RenderTarget::Window(window_id))[0];

    let ctx = rt.root_context(document);
    match event {
        SpawnWindow::MainWindow => {
            MainWindow { state }.mount(&ctx);
        }
        SpawnWindow::Modules => {
            Modules { state }.mount(&ctx);
        }
        SpawnWindow::OpenModule => {
            OpenModule {
                handle: state.handle,
            }
            .mount(&ctx);
        }
        SpawnWindow::Error(msg) => {
            Error { message: msg }.mount(&ctx);
        }
        SpawnWindow::CreateModule => {
            CreateModule {
                modules: state.modules,
            }
            .mount(&ctx);
        }
        SpawnWindow::EditModule(id) => {
            EditModule {
                modules: state.modules,
                id: Some(id),
            }
            .mount(&ctx);
        }
        SpawnWindow::Records => {
            Records { state }.mount(&ctx);
        }
        SpawnWindow::EditWorld => {
            let inner = EditWorldWindow::new(&ctx, state);

            return Window {
                document,
                inner: Some(Box::new(inner)),
            };
        }
        SpawnWindow::EditRecord(kind, id) => {
            EditRecord { kind, id, state }.mount(&ctx);
        }
        SpawnWindow::EditPrefab(edit_state) => {
            let inner = EditPrefabWindow::new(&ctx, edit_state, modules);

            return Window {
                document,
                inner: Some(Box::new(inner)),
            };
        }
        SpawnWindow::Explorer(on_open) => {
            Explorer::new(on_open).mount(&ctx);
        }
    }

    Window {
        document,
        inner: None,
    }
}

#[derive(Clone, Debug)]
pub enum SpawnWindow {
    MainWindow,
    Modules,
    CreateModule,
    EditModule(ModuleId),
    OpenModule,
    Records,
    EditWorld,
    Error(String),
    EditRecord(RecordKind, Option<RecordReference>),
    EditPrefab(Arc<Mutex<EditState>>),
    Explorer(Callback<Vec<Entry>>),
}
