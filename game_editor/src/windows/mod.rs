mod create_module;
mod edit_prefab;
mod error;
pub mod main_window;
pub mod modules;
mod open_module;
mod record;
mod records;
mod world;

use std::sync::Arc;

use create_module::EditModule;
use game_common::world::World;
use game_data::record::RecordKind;
use game_render::camera::RenderTarget;
use game_render::options::MainPassOptions;
use game_render::Renderer;
use game_tracing::trace_span;
use game_ui::reactive::DocumentId;
use game_ui::widgets::Widget;
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
use crate::windows::create_module::CreateModule;
use crate::windows::error::Error;

use self::main_window::MainWindow;
use self::open_module::OpenModule;
use self::world::WorldWindowState;

pub enum Window {
    View(DocumentId, WorldWindowState),
    Other(DocumentId),
}

impl Window {
    // pub fn doc(&self) -> Option<Document> {
    //     match self {
    //         Self::View(doc, _) => Some(doc.clone()),
    //         Self::Other(doc) => Some(doc.clone()),
    //     }
    // }

    pub fn handle_event(
        &mut self,
        world: &mut World,
        renderer: &mut Renderer,
        event: WindowEvent,
        id: WindowId,
    ) {
        let _span = trace_span!("Window::handle_event").entered();

        match self {
            Self::View(_, window) => window.handle_event(world, event, id, renderer),
            _ => (),
        }
    }

    pub fn update(&mut self, world: &mut World, renderer: &mut Renderer) {
        let _span = trace_span!("Window::update").entered();

        let mut options = MainPassOptions::default();

        match self {
            Self::View(_, w) => {
                w.update(world, &mut options);
                renderer.set_options(options);
            }
            _ => (),
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
        SpawnWindow::View => {
            let window =
                WorldWindowState::new(&ctx, window_id, world, modules, None, World::default());
            return Window::View(document, window);
        }
        SpawnWindow::EditRecord(kind, id) => {
            EditRecord { kind, id, state }.mount(&ctx);
        }
        SpawnWindow::EditPrefab(edit_state) => {
            let prefab_world = edit_prefab::load_prefab(&edit_state);

            let window = WorldWindowState::new(
                &ctx,
                window_id,
                world,
                modules,
                Some(edit_prefab::on_world_change_callback(edit_state)),
                prefab_world,
            );

            return Window::View(document, window);
        }
    }

    Window::Other(document)
}

#[derive(Clone, Debug)]
pub enum SpawnWindow {
    MainWindow,
    Modules,
    CreateModule,
    EditModule(ModuleId),
    OpenModule,
    Records,
    View,
    Error(String),
    EditRecord(RecordKind, Option<RecordReference>),
    EditPrefab(Arc<Mutex<EditState>>),
}
