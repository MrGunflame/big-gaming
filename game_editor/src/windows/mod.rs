mod create_module;
mod error;
pub mod main_window;
pub mod modules;
mod open_module;
mod record;
mod records;
mod world;

use game_common::module::ModuleId;
use game_data::record::{Record, RecordKind};
use game_render::Renderer;
use game_scene::Scenes;
use game_ui::reactive::{Document, Runtime};
use game_window::events::WindowEvent;
use game_window::windows::WindowId;

use crate::state::EditorState;
use crate::windows::create_module::CreateModule;
use crate::windows::error::Error;
use crate::windows::record::EditRecord;
use crate::windows::records::Records;

use self::main_window::MainWindow;
use self::modules::Modules;
use self::open_module::OpenModule;
use self::record::CreateRecord;
use self::world::WorldWindowState;

pub enum Window {
    View(WorldWindowState),
    Other(Document),
}

impl Window {
    pub fn doc(&self) -> Option<Document> {
        match self {
            Self::View(_) => None,
            Self::Other(doc) => Some(doc.clone()),
        }
    }

    pub fn handle_event(&mut self, renderer: &mut Renderer, event: WindowEvent, id: WindowId) {
        match self {
            Self::View(window) => window.handle_event(renderer, event, id),
            _ => (),
        }
    }
}

pub fn spawn_window(
    renderer: &mut Renderer,
    scenes: &mut Scenes,
    state: EditorState,
    rt: Runtime,
    event: SpawnWindow,
    window_id: WindowId,
) -> Window {
    let document = Document::new(rt);

    let cx = document.root_scope();
    match event {
        SpawnWindow::MainWindow => {
            cx.append(MainWindow { state });
        }
        SpawnWindow::Modules => {
            cx.append(Modules { state });
        }
        SpawnWindow::OpenModule => {
            cx.append(OpenModule {
                handle: state.handle,
            });
        }
        SpawnWindow::CreateModule => {
            cx.append(CreateModule {
                modules: state.modules,
            });
        }
        SpawnWindow::Error(msg) => {
            cx.append(Error { message: msg });
        }
        SpawnWindow::Records => {
            cx.append(Records { state });
        }
        SpawnWindow::CreateRecord(kind) => {
            cx.append(CreateRecord {
                kind,
                records: state.records,
                modules: state.modules,
            });
        }
        SpawnWindow::EditRecord(module_id, record) => {
            cx.append(EditRecord {
                record,
                module_id,
                records: state.records,
                modules: state.modules,
            });
        }
        SpawnWindow::View => {
            let window = world::WorldWindowState::new(renderer, window_id, scenes);
            return Window::View(window);
        }
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
