use std::sync::Arc;

use game_common::module::Module;
use game_common::reflection::RecordDescriptor;
use game_data::record::{Record, RecordKind};
use game_ui::reactive::Context;
use game_ui::widgets::{Button, Callback, Container, Input, Selection, Text, Widget};
use game_wasm::record::{ModuleId, RecordId};
use game_wasm::world::RecordReference;
use parking_lot::Mutex;

use crate::state::EditorState;

use super::SpawnWindow;

#[derive(Debug)]
pub struct EditRecord {
    pub kind: RecordKind,
    pub id: Option<RecordReference>,
    pub state: EditorState,
}

impl Widget for EditRecord {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let record = if let Some(id) = self.id {
            self.state.records.get(id.module, id.record).unwrap()
        } else {
            Record {
                id: RecordId(0),
                kind: self.kind,
                name: String::new(),
                description: String::new(),
                data: Vec::new(),
            }
        };

        let edit_state = Arc::new(Mutex::new(EditState {
            id: self.id,
            record,
            module: None,
        }));

        {
            let modules: Vec<_> = self
                .state
                .modules
                .iter()
                .filter(|module| module.capabilities.write())
                .map(|module| module.module)
                .collect();

            let options = modules.iter().map(|module| module.name.clone()).collect();

            Selection {
                options,
                on_change: Callback::from({
                    let edit_state = edit_state.clone();
                    move |index| {
                        let module: &Module = &modules[index];
                        let mut edit_state = edit_state.try_lock().unwrap();
                        edit_state.module = Some(module.id);
                    }
                }),
            }
            .mount(&root);
        }

        Input::new()
            .on_change({
                let edit_state = edit_state.clone();
                move |value| {
                    let mut edit_state = edit_state.try_lock().unwrap();
                    edit_state.record.name = value;
                }
            })
            .mount(&root);
        Input::new()
            .on_change({
                let edit_state = edit_state.clone();
                move |value| {
                    let mut edit_state = edit_state.lock();
                    edit_state.record.description = value;
                }
            })
            .mount(&root);

        match self.kind {
            RecordKind::COMPONENT => {
                EditComponentRecord {}.mount(&root);
            }
            RecordKind::RECORD => {
                let state = self.state.clone();
                let edit_state = edit_state.clone();

                EditRecordRecord { state, edit_state }.mount(&root);
            }
            RecordKind::PREFAB => {
                let state = self.state.clone();
                let edit_state = edit_state.clone();

                EditPrefabRecord {
                    edit_state: edit_state,
                    state,
                }
                .mount(&root);
            }
            RecordKind::SCRIPT => {
                let edit_state = edit_state.clone();

                EditScript { edit_state }.mount(&root);
            }
            _ => todo!(),
        }

        let button = Button::new()
            .on_click(move |()| {
                let mut edit_state = edit_state.lock();

                let Some(module) = edit_state.module else {
                    return;
                };

                // We should avoid records with an empty name.
                if edit_state.record.name.is_empty() {
                    return;
                }

                edit_state.record.id = if let Some(id) = edit_state.id {
                    id.record
                } else {
                    self.state.records.take_id(module)
                };

                tracing::debug!("create record with id {}", edit_state.record.id);
                self.state.records.insert(module, edit_state.record.clone());
            })
            .mount(&root);
        Text::new("OK").mount(&button);

        root
    }
}

#[derive(Debug)]
struct EditComponentRecord {}

impl Widget for EditComponentRecord {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);
        Text::new("TODO").mount(&root);
        root
    }
}

#[derive(Debug)]
struct EditRecordRecord {
    state: EditorState,
    edit_state: Arc<Mutex<EditState>>,
}

impl Widget for EditRecordRecord {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let components: Vec<_> = self
            .state
            .records
            .iter()
            .filter(|(_, record)| record.kind == RecordKind::COMPONENT)
            .collect();

        let options = components
            .iter()
            .map(|(_, record)| record.name.clone())
            .collect();

        Selection {
            options,
            on_change: Callback::from(move |index| {
                let (module, record): &(ModuleId, Record) = &components[index];

                let descriptor = RecordDescriptor {
                    component: RecordReference {
                        module: *module,
                        record: record.id,
                    },
                    // TODO: Allow key customization.
                    keys: Vec::new(),
                };

                let mut edit_state = self.edit_state.lock();
                edit_state.record.data = descriptor.to_bytes();
            }),
        }
        .mount(&root);

        root
    }
}

#[derive(Clone, Debug)]
pub(super) struct EditState {
    pub module: Option<ModuleId>,
    pub id: Option<RecordReference>,
    pub record: Record,
}

#[derive(Clone, Debug)]
struct EditPrefabRecord {
    state: EditorState,
    edit_state: Arc<Mutex<EditState>>,
}

impl Widget for EditPrefabRecord {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let button = Button::new()
            .on_click(move |()| {
                let edit_state = self.edit_state.clone();
                self.state
                    .spawn_windows
                    .send(SpawnWindow::EditPrefab(edit_state))
                    .unwrap();
            })
            .mount(&root);
        Text::new("Edit in 3D").mount(&button);

        root
    }
}

#[derive(Clone, Debug)]
struct EditScript {
    edit_state: Arc<Mutex<EditState>>,
}

impl Widget for EditScript {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        Input::new()
            .on_change(move |path: String| {
                self.edit_state.lock().record.data = path.into();
            })
            .mount(&root);

        root
    }
}
