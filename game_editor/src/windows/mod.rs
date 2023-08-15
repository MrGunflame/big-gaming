mod create_module;
mod error;
pub mod modules;
mod open_module;
mod record;
mod records;
//mod view;

use game_asset::Assets;
use game_common::module::ModuleId;
use game_data::record::{Record, RecordKind};
use game_render::mesh::Mesh;
use game_render::pbr::PbrMaterial;
use game_render::texture::Images;
use game_scene::Scenes;
use game_ui::events::Events;
use game_ui::reactive::{Document, Runtime};
use game_ui::render::layout::LayoutTree;
use game_ui::view;

use crate::backend::Handle;
use crate::state::EditorState;

use self::create_module::*;
use self::error::*;
use self::modules::*;
use self::open_module::*;
use self::record::*;
use self::records::*;

pub fn spawn_window(state: EditorState, rt: Runtime, event: SpawnWindow) -> Document {
    let document = Document::new(rt);

    let cx = document.root_scope();
    match event {
        SpawnWindow::Modules => {
            view! {
                cx,
                <Modules state={state.clone()}>
                </Modules>
            };
        }
        SpawnWindow::OpenModule => {
            view! {
                cx,
                <OpenModule handle={state.handle.clone()}>
                </OpenModule>
            };
        }
        SpawnWindow::CreateModule => {
            view! {
                cx,
                <CreateModule modules={state.modules.clone()}>
                </CreateModule>
            };
        }
        SpawnWindow::Error(msg) => {
            view! {
                cx,
                <Error message={&msg}>
                </Error>
            };
        }
        SpawnWindow::Records => {
            view! {
                cx,
                <Records state={state.clone()}>
                </Records>
            };
        }
        SpawnWindow::CreateRecord(kind) => {
            view! {
                cx,
                <CreateRecord kind={kind} records={state.records.clone()} modules={state.modules.clone()}>
                </CreateRecord>
            };
        }
        SpawnWindow::EditRecord(module_id, record) => {
            view! {
                cx,
                <EditRecord record={record.clone()} modules={state.modules.clone()} records={state.records.clone()} module_id={module_id}>
                </EditRecord>
            };
        }
        _ => todo!(),
    }

    document
}

#[derive(Clone, Debug)]
pub enum SpawnWindow {
    Modules,
    CreateModule,
    OpenModule,
    Records,
    View,
    Error(String),
    CreateRecord(RecordKind),
    EditRecord(ModuleId, Record),
}
