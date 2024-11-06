use game_common::module::Module;
use game_common::reflection::RecordDescriptor;
use game_data::record::{Record, RecordKind};
use game_ui::runtime::reactive::WriteSignal;
use game_ui::runtime::Context;
use game_ui::widgets::{Button, Callback, Container, Input, Selection, Text, Widget};
use game_wasm::record::{ModuleId, RecordId};
use game_wasm::world::RecordReference;

use crate::state::EditorState;
use crate::widgets::explorer::Entry;

use super::SpawnWindow;

#[derive(Debug)]
pub struct EditRecord {
    pub kind: RecordKind,
    pub id: Option<RecordReference>,
    pub state: EditorState,
}

impl Widget for EditRecord {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);

        let record = match self.id {
            Some(id) => match self.state.records.get(id.module, id.record) {
                Some(record) => record,
                // If the `id` does not refer to a valid record we likely
                // got outraced by a delete operation of the record.
                None => {
                    // TODO: Figure out how to handle this case.
                    // Should we create an entirely new record or recreate
                    // a "new" record with the same id as the old one?
                    todo!()
                }
            },
            None => Record {
                id: RecordId(0),
                kind: self.kind,
                name: String::new(),
                description: String::new(),
                data: Vec::new(),
            },
        };

        let (edit_state, set_edit_state) = root.runtime().reactive().create_signal(EditState {
            id: self.id,
            record,
            module: self.id.map(|id| id.module),
        });

        Text::new("Module").mount(&root);

        let modules: Vec<_> = self
            .state
            .modules
            .iter()
            .filter(|module| module.capabilities.write())
            .map(|module| module.module)
            .collect();

        let selected_module = match self.id {
            Some(id) => modules.iter().position(|module| module.id == id.module),
            None => None,
        };

        let options = modules.iter().map(|module| module.name.clone()).collect();

        let mut selection = Selection::new(options).on_change({
            let set_edit_state = set_edit_state.clone();

            move |index| {
                let module: &Module = &modules[index];

                set_edit_state.update(|state| {
                    state.module = Some(module.id);
                });
            }
        });

        if let Some(index) = selected_module {
            selection = selection.value(index);
        }

        selection.mount(&root);

        Text::new("Name").mount(&root);
        Input::new()
            .on_change({
                let set_edit_state = set_edit_state.clone();

                move |value| {
                    set_edit_state.update(|state| {
                        state.record.name = value;
                    });
                }
            })
            .value(edit_state.with(|state| state.record.name.clone()))
            .mount(&root);

        Text::new("Description").mount(&root);
        Input::new()
            .on_change({
                let set_edit_state = set_edit_state.clone();

                move |value| {
                    set_edit_state.update(|state| {
                        state.record.description = value;
                    });
                }
            })
            .value(edit_state.with(|state| state.record.description.clone()))
            .mount(&root);

        match self.kind {
            RecordKind::COMPONENT => {
                EditComponentRecord {}.mount(&root);
            }
            RecordKind::RECORD => {
                EditRecordRecord {
                    state: self.state.clone(),
                    edit_state: set_edit_state,
                }
                .mount(&root);
            }
            RecordKind::PREFAB => {
                EditPrefabRecord {
                    state: self.state.clone(),
                    edit_state: set_edit_state,
                }
                .mount(&root);
            }
            RecordKind::SCRIPT => {
                EditScript {
                    edit_state: set_edit_state,
                }
                .mount(&root);
            }
            RecordKind::RESOURCE => {
                EditResource {
                    edit_state: set_edit_state,
                    state: self.state.clone(),
                }
                .mount(&root);
            }
            _ => todo!(),
        }

        let button = Button::new()
            .on_click(move |()| {
                let mut state = edit_state.get();

                let Some(module) = state.module else {
                    return;
                };

                // We should avoid records with an empty name.
                if state.record.name.is_empty() {
                    return;
                }

                // If we are editing an existing record we already have
                // and ID, otherwise we must create a new one.
                state.record.id = if let Some(id) = state.id {
                    id.record
                } else {
                    self.state.records.take_id(module)
                };

                tracing::debug!("create record with id {}", state.record.id);
                self.state.records.insert(module, state.record);
            })
            .mount(&root);
        Text::new("OK").mount(&button);

        root
    }
}

#[derive(Debug)]
struct EditComponentRecord {}

impl Widget for EditComponentRecord {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);
        Text::new("TODO").mount(&root);
        root
    }
}

#[derive(Debug)]
struct EditRecordRecord {
    state: EditorState,
    edit_state: WriteSignal<EditState>,
}

impl Widget for EditRecordRecord {
    fn mount(self, parent: &Context) -> Context {
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

        Text::new("Descriptor").mount(&root);
        Selection::new(options)
            .on_change(move |index| {
                let (module, record): &(ModuleId, Record) = &components[index];

                let descriptor = RecordDescriptor {
                    component: RecordReference {
                        module: *module,
                        record: record.id,
                    },
                    // TODO: Allow key customization.
                    keys: Vec::new(),
                };

                self.edit_state.update(|state| {
                    state.record.data = descriptor.to_bytes();
                });
            })
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
    edit_state: WriteSignal<EditState>,
}

impl Widget for EditPrefabRecord {
    fn mount(self, parent: &Context) -> Context {
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
    edit_state: WriteSignal<EditState>,
}

impl Widget for EditScript {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);

        Text::new("Path").mount(&root);
        Input::new()
            .on_change(move |path: String| {
                self.edit_state.update(|state| {
                    state.record.data = path.into();
                });
            })
            .mount(&root);

        root
    }
}

struct EditResource {
    state: EditorState,
    edit_state: WriteSignal<EditState>,
}

impl Widget for EditResource {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);

        let button = Button::new()
            .on_click(move |_| {
                let edit_state = self.edit_state.clone();
                let on_open = Callback::from(move |entries: Vec<Entry>| {
                    let Some(entry) = entries.get(0) else {
                        return;
                    };

                    match std::fs::read(&entry.path) {
                        Ok(data) => {
                            edit_state.update(|state| {
                                state.record.data = data;
                            });
                        }
                        Err(err) => {
                            tracing::error!("failed to load record from file: {}", err);
                        }
                    }
                });

                self.state
                    .spawn_windows
                    .send(SpawnWindow::Explorer(on_open))
                    .unwrap();
            })
            .mount(&root);
        Text::new("Open File").mount(&button);

        root
    }
}
