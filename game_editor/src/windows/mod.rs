mod create_module;
mod error;
pub mod main_window;
pub mod modules;
mod open_module;
// // mod record;
mod records;
mod world;

use game_common::module::ModuleId;
use game_common::world::World;
use game_data::record::{Record, RecordKind};
use game_render::camera::RenderTarget;
use game_render::Renderer;
use game_ui::reactive::DocumentId;
use game_ui::widgets::Widget;
use game_ui::UiState;
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use modules::Modules;
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
        match self {
            Self::View(_, window) => window.handle_event(world, event, id, renderer),
            _ => (),
        }
    }

    pub fn update(&mut self, world: &mut World, renderer: &mut Renderer) {
        match self {
            Self::View(_, w) => w.update(world),
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
        SpawnWindow::Records => {
            Records { state }.mount(&ctx);
        }
        SpawnWindow::View => {
            let window = world::WorldWindowState::new(&ctx, window_id, world, modules);
            return Window::View(document, window);
        }
        _ => todo!(),
        // SpawnWindow::CreateRecord(kind) => {
        //     // cx.append(CreateRecord {
        //     //     kind,
        //     //     records: state.records,
        //     //     modules: state.modules,
        //     // });
        // }
        // SpawnWindow::EditRecord(module_id, record) => {
        //     // cx.append(EditRecord {
        //     //     record,
        //     //     module_id,
        //     //     records: state.records,
        //     //     modules: state.modules,
        //     // });
        // }
    }

    Window::Other(document)
}

#[derive(Clone, Debug)]
pub enum SpawnWindow {
    MainWindow,
    Modules,
    CreateModule,
    OpenModule,
    Records,
    View,
    Error(String),
    CreateRecord(RecordKind),
    EditRecord(ModuleId, Record),
}
