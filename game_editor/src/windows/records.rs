//! The template data editor.

use std::path::PathBuf;

use bevy::prelude::{Commands, Component, Entity, EventWriter, Query, ResMut};
use bevy_egui::egui::panel::Side;
use bevy_egui::egui::{CentralPanel, SidePanel, TextEdit};
use bevy_egui::EguiContext;
use egui_extras::{Column, TableBuilder};
use game_common::module::ModuleId;
use game_common::units::Mass;
use game_data::components::actions::ActionRecord;
use game_data::components::item::ItemRecord;
use game_data::record::{Record, RecordBody, RecordId, RecordKind, RecordReference};
use game_data::uri::Uri;

use crate::state::module::Modules;
use crate::state::record::Records;

const CATEGORIES: &[RecordKind] = &[RecordKind::Item, RecordKind::Action];

use super::SpawnWindow;
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordsWindowPlugin;

impl bevy::prelude::Plugin for RecordsWindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(render_window);

        app.add_system(render_record_windows);
        app.add_system(render_create_record_windows);
    }
}

#[derive(Clone, Debug, Component)]
pub struct RecordsWindow {
    state: State,
    category: RecordKind,
}

impl RecordsWindow {
    pub fn new() -> Self {
        Self {
            state: State {
                search: String::new(),
                categories: [false; 1],
            },
            category: RecordKind::Item,
        }
    }
}

fn render_window(
    mut windows: Query<(&mut EguiContext, &mut RecordsWindow)>,
    mut events: EventWriter<SpawnWindow>,
    mut records: ResMut<Records>,
) {
    for (mut ctx, mut window) in &mut windows {
        // Reborrow Mut<..> as &mut ..
        let window = window.as_mut();

        SidePanel::new(Side::Left, "form_selector").show(ctx.get_mut(), |ui| {
            ui.add(TextEdit::singleline(&mut window.state.search).hint_text("Search"));

            for category in CATEGORIES {
                if ui.button(category_str(*category)).clicked() {
                    window.category = *category;
                }
            }
        });

        let count = 4 + match window.category {
            RecordKind::Item => 3,
            RecordKind::Action => 1,
            _ => 0,
        };

        CentralPanel::default().show(ctx.get_mut(), |ui| {
            TableBuilder::new(ui)
                .columns(Column::remainder().resizable(true), count)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Module ID");
                    });
                    header.col(|ui| {
                        ui.heading("Record ID");
                    });
                    header.col(|ui| {
                        ui.heading("Editor Name");
                    });

                    match window.category {
                        RecordKind::Item => {
                            header.col(|ui| {
                                ui.heading("Mass");
                            });
                            header.col(|ui| {
                                ui.heading("Value");
                            });
                            header.col(|ui| {
                                ui.heading("Actions");
                            });
                        }
                        RecordKind::Action => {
                            header.col(|ui| {
                                ui.heading("Description");
                            });
                        }
                    }

                    header.col(|ui| {
                        ui.heading("EDIT:w");
                    });
                })
                .body(|mut body| {
                    for (module, record) in records.iter() {
                        if record.body.kind() != window.category {
                            continue;
                        }

                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(module.to_string());
                            });
                            row.col(|ui| {
                                ui.label(record.id.to_string());
                            });
                            row.col(|ui| {
                                ui.label(&record.name);
                            });

                            match &record.body {
                                RecordBody::Item(item) => {
                                    row.col(|ui| {
                                        ui.label(item.mass.to_grams().to_string());
                                    });
                                    row.col(|ui| {
                                        ui.label(item.value.to_string());
                                    });
                                    row.col(|ui| {
                                        ui.label(item.actions.len().to_string());
                                    });
                                }
                                RecordBody::Action(action) => {
                                    row.col(|ui| {
                                        ui.label(&action.description);
                                    });
                                }
                            }

                            row.col(|ui| {
                                if ui.button("Edit").clicked() {
                                    events.send(SpawnWindow::Record(module, record.id));
                                }
                            });
                        });
                    }
                });

            if ui.button("Add").clicked() {
                events.send(SpawnWindow::CreateRecord(window.category));
            }
        });
    }
}

fn category_str(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Item => "Items",
        RecordKind::Action => "Actions",
    }
}

#[derive(Clone, Debug)]
struct State {
    search: String,
    categories: [bool; 1],
}

#[derive(Clone, Debug, Component)]
pub struct RecordWindow {
    pub module: ModuleId,
    pub id: RecordId,
    pub record: Option<Record>,
    // TODO: RecordId
    pub add_action: u32,
}

fn render_record_windows(
    mut commands: Commands,
    mut windows: Query<(Entity, &mut EguiContext, &mut RecordWindow)>,
    mut records: ResMut<Records>,
) {
    for (entity, mut ctx, mut state) in &mut windows {
        let mut record = match state.record.take() {
            Some(r) => r,
            None => records.get(state.module, state.id).unwrap().clone(),
        };

        let mut changed = false;

        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.heading("Metadata");

            ui.label("Record ID");
            if ui
                .add_enabled(
                    false,
                    TextEdit::singleline(&mut record.id.to_string()).interactive(false),
                )
                .changed()
            {
                changed = true;
            }

            ui.label("Name");
            if ui.add(TextEdit::singleline(&mut record.name)).changed() {
                changed = true;
            }

            match &mut record.body {
                RecordBody::Item(item) => {
                    ui.heading("Item");

                    ui.label("Mass (g)");

                    let mut mass = item.mass.to_grams().to_string();
                    if ui.add(TextEdit::singleline(&mut mass)).changed() {
                        let val = mass.parse::<u32>().unwrap_or_default();
                        item.mass = Mass::from_grams(val);
                        changed = true;
                    }

                    ui.label("Value");

                    let mut value = item.value.to_string();
                    if ui.add(TextEdit::singleline(&mut value)).changed() {
                        let val = value.parse::<u64>().unwrap_or_default();
                        item.value = val;
                        changed = true;
                    }

                    ui.label("URI (relative)");

                    let mut uri = item.uri.as_ref().to_str().unwrap().to_owned();
                    if ui.add(TextEdit::singleline(&mut uri)).changed() {
                        item.uri = Uri::from(PathBuf::from(uri));
                        changed = true;
                    }

                    ui.label("Actions");

                    let mut index = 0;
                    while index < item.actions.len() {
                        let action =
                            records.get(item.actions[index].module, item.actions[index].record);

                        let text = match action {
                            Some(action) => format!("{} ({})", action.name, action.id),
                            None => format!("Invalid reference ({})", item.actions[index]),
                        };

                        ui.label(text);
                        if ui.button("Delete").clicked() {
                            item.actions.remove(index);
                            continue;
                        }

                        index += 1;
                    }

                    ui.label("Add Action:");
                    let mut add_action = state.add_action.to_string();
                    if ui.add(TextEdit::singleline(&mut add_action)).changed() {
                        state.add_action = add_action.parse().unwrap_or_default();
                        changed = true;
                    }

                    if ui.button("Add").clicked() {
                        if records
                            .get(state.module, RecordId(state.add_action))
                            .is_some()
                        {
                            item.actions.push(RecordReference {
                                module: state.module,
                                record: RecordId(state.add_action),
                            });
                        }
                    }
                }
                RecordBody::Action(action) => {
                    ui.label("Description");

                    if ui
                        .add(TextEdit::multiline(&mut action.description))
                        .changed()
                    {
                        changed = true;
                    }
                }
            }

            if ui.button("Ok").clicked() {
                records.insert(state.module, record);
                commands.entity(entity).despawn();
            } else {
                state.record = Some(record);
            }

            if ui.button("Cancel").clicked() {
                commands.entity(entity).despawn();
            }
        });
    }
}

#[derive(Clone, Debug, Component)]
pub struct CreateRecordWindow {
    module: Option<ModuleId>,
    id: RecordId,
    kind: RecordKind,
}

impl CreateRecordWindow {
    pub fn new(kind: RecordKind) -> Self {
        Self {
            module: None,
            id: RecordId(0),
            kind,
        }
    }
}

fn render_create_record_windows(
    mut commands: Commands,
    mut windows: Query<(Entity, &mut EguiContext, &mut CreateRecordWindow)>,
    modules: ResMut<Modules>,
    mut records: ResMut<Records>,
) {
    for (entity, mut ctx, mut state) in &mut windows {
        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.heading("Create Record");

            ui.label("Module");
            if modules.is_empty() {
                ui.label("No modules opened");
            } else {
                for m in modules.iter() {
                    let id = m.module.id;

                    if ui
                        .radio(
                            state.module.map(|m| m == id).unwrap_or(false),
                            id.to_string(),
                        )
                        .clicked()
                    {
                        state.module = Some(id);
                    }
                }
            }

            if ui.button("Ok").clicked() {
                if let Some(module_id) = state.module {
                    records.push(
                        module_id,
                        Record {
                            id: RecordId(0),
                            name: String::new(),
                            body: match state.kind {
                                RecordKind::Item => RecordBody::Item(ItemRecord {
                                    mass: Mass::new(),
                                    value: 0,
                                    uri: Uri::new(),
                                    actions: Vec::new(),
                                }),
                                RecordKind::Action => RecordBody::Action(ActionRecord {
                                    description: String::new(),
                                }),
                            },
                        },
                    );
                }

                commands.entity(entity).despawn();
            }
        });
    }
}
