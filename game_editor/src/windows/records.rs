use std::sync::Arc;

use game_common::components::components::RawComponent;
use game_common::reflection::editor::ComponentEditor;
use game_common::reflection::{ComponentDescriptor, FieldIndex, FieldKind, RecordDescriptor};
use game_data::record::{Record, RecordKind};
use game_ui::reactive::Context;
use game_ui::style::{Background, Direction, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};
use game_wasm::record::ModuleId;
use game_wasm::world::RecordReference;
use image::Rgba;
use parking_lot::Mutex;

use crate::widgets::entries::*;

use crate::state::EditorState;

use super::SpawnWindow;

// const SELECTED_COLOR: Background = Background::Color(Rgba([0x04, 0x7d, 0xd3, 0xFF]));

const BACKGROUND_COLOR: [Background; 2] = [
    Background::Color(Rgba([0x50, 0x50, 0x50, 0xFF])),
    Background::Color(Rgba([0x2a, 0x2a, 0x2a, 0xFF])),
];

pub struct Records {
    pub state: EditorState,
}

impl Widget for Records {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new()
            .style(Style {
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(parent);

        let state = Arc::new(Mutex::new(State {
            selected: RecordKind::COMPONENT,
            root: root.clone(),
            record_list: None,
        }));

        SidePanel {
            state: self.state.clone(),
            ui_state: state.clone(),
        }
        .mount(&root);

        let record_list = RecordList {
            state: self.state,
            selected: RecordKind::COMPONENT,
        }
        .mount(&root);
        state.try_lock().unwrap().record_list = Some(record_list);

        root
    }
}

struct SidePanel {
    state: EditorState,
    ui_state: Arc<Mutex<State>>,
}

impl Widget for SidePanel {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let records: Vec<_> = self
            .state
            .records
            .iter()
            .filter(|(_, record)| record.kind == RecordKind::RECORD)
            .map(|(module, record)| {
                (
                    RecordReference {
                        module,
                        record: record.id,
                    },
                    record.name.clone(),
                )
            })
            .collect();

        for (id, name) in records {
            let button = Button::new()
                .style(Style {
                    ..Default::default()
                })
                .on_click({
                    let ui_state = self.ui_state.clone();
                    let state = self.state.clone();
                    move |()| {
                        let mut ui_state = ui_state.try_lock().unwrap();
                        ui_state.selected = RecordKind(id);

                        if let Some(ctx) = ui_state.record_list.take() {
                            ctx.remove(ctx.node().unwrap());
                        }

                        let record_list = RecordList {
                            state: state.clone(),
                            selected: ui_state.selected,
                        }
                        .mount(&ui_state.root);
                        ui_state.record_list = Some(record_list);
                    }
                })
                .mount(&root);
            Text::new(name).mount(&button);
        }

        root
    }
}

struct RecordList {
    state: EditorState,
    selected: RecordKind,
}

impl Widget for RecordList {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let keys = vec!["ID".to_owned(), "Name".to_owned()];
        let mut records = Vec::new();
        let mut record_descriptor = None;
        for (module, record) in self.state.records.iter() {
            if record.kind == RecordKind::RECORD {
                record_descriptor = Some(RecordDescriptor::from_bytes(&record.data));
            }

            if record.kind != self.selected {
                continue;
            }

            records.push((module, record.clone()));
        }

        let record_descriptor = record_descriptor.unwrap();
        let mut component_descriptor = None;
        for (module, record) in self.state.records.iter() {
            if record.kind == RecordKind::COMPONENT
                && module == record_descriptor.component.module
                && record.id == record_descriptor.component.record
            {
                let descriptor = ComponentDescriptor::from_bytes(&record.data);
                component_descriptor = Some(descriptor);
                break;
            }
        }

        let mut entries = Vec::new();
        if let Some(component_descriptor) = component_descriptor {
            for (_, record) in &records {
                let component = RawComponent::new(record.data.clone(), vec![]);
                let editor = ComponentEditor::new(&component_descriptor, component);

                let mut keys = vec![record.id.to_string(), record.name.clone()];
                for field in &record_descriptor.keys {
                    let key = format_component_field(&editor, *field).unwrap_or_default();
                    keys.push(key);
                }

                entries.push(keys);
            }
        } else {
            for (_, record) in &records {
                entries.push(vec![record.id.to_string(), record.name.clone()]);
            }
        }

        let data = EntriesData {
            keys,
            entries,
            add_entry: Some(Callback::from({
                let state = self.state.clone();

                move |()| {
                    state
                        .spawn_windows
                        .send(SpawnWindow::EditRecord(self.selected, None))
                        .unwrap();
                }
            })),
            edit_entry: Some(Callback::from({
                let state = self.state.clone();

                move |index| {
                    let (module, record): &(ModuleId, Record) = &records[index];

                    state
                        .spawn_windows
                        .send(SpawnWindow::EditRecord(
                            self.selected,
                            Some(RecordReference {
                                module: *module,
                                record: record.id,
                            }),
                        ))
                        .unwrap();
                }
            })),
            remove_entry: None,
        };
        Entries { data }.mount(&root);

        root
    }
}

fn format_component_field(editor: &ComponentEditor<'_>, field: FieldIndex) -> Option<String> {
    let data = editor.get(field)?;
    let field = editor.descriptor().get(field)?;

    match field.kind {
        FieldKind::Int(field) => match (data.len(), field.is_signed) {
            (1, false) => Some(data[0].to_string()),
            (1, true) => Some((data[0] as i8).to_string()),
            (2, false) => Some(u16::from_le_bytes(data.try_into().unwrap()).to_string()),
            (2, true) => Some(i16::from_le_bytes(data.try_into().unwrap()).to_string()),
            (4, false) => Some(u32::from_le_bytes(data.try_into().unwrap()).to_string()),
            (4, true) => Some(i32::from_le_bytes(data.try_into().unwrap()).to_string()),
            (8, false) => Some(u64::from_le_bytes(data.try_into().unwrap()).to_string()),
            (8, true) => Some(i64::from_le_bytes(data.try_into().unwrap()).to_string()),
            _ => todo!(),
        },
        FieldKind::Float(_) => match data.len() {
            4 => Some(f32::from_le_bytes(data.try_into().unwrap()).to_string()),
            8 => Some(f64::from_le_bytes(data.try_into().unwrap()).to_string()),
            _ => todo!(),
        },
        _ => todo!(),
    }
}

struct State {
    selected: RecordKind,
    record_list: Option<Context<()>>,
    root: Context<()>,
}
