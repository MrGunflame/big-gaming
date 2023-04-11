//! The template data editor.

use bevy::prelude::{Component, EventWriter, Query};
use bevy_egui::egui::panel::Side;
use bevy_egui::egui::{Align, CentralPanel, Layout, SidePanel, TextEdit};
use bevy_egui::EguiContext;
use game_common::module::ModuleId;
use game_common::units::Mass;
use game_data::record::{ItemRecord, Record, RecordBody, RecordId};

use crate::state::module::Records;

use super::SpawnWindow;
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordsWindowPlugin;

impl bevy::prelude::Plugin for RecordsWindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(render_window);

        app.add_system(render_record_windows);
    }
}

#[derive(Clone, Debug, Component)]
pub struct RecordsWindow {
    module: ModuleId,
    records: Records,
    state: State,
}

impl RecordsWindow {
    pub fn new(module: ModuleId, records: Records) -> Self {
        Self {
            module,
            records,
            state: State {
                search: String::new(),
                categories: [false; 1],
            },
        }
    }
}

fn render_window(
    mut windows: Query<(&mut EguiContext, &mut RecordsWindow)>,
    mut events: EventWriter<SpawnWindow>,
) {
    for (mut ctx, mut window) in &mut windows {
        // Reborrow Mut<..> as &mut ..
        let window = window.as_mut();

        SidePanel::new(Side::Left, "form_selector").show(ctx.get_mut(), |ui| {
            ui.add(TextEdit::singleline(&mut window.state.search).hint_text("Search"));

            for category in &[Category::Items] {
                ui.label(category.as_str());
            }
        });

        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.label("ID");
                ui.label("Name");
                ui.label("Mass");
                ui.label("Value");
            });

            for record in window.records.iter() {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.label(record.id.to_string());
                    ui.label(record.name);

                    match record.body {
                        RecordBody::Item(item) => {
                            ui.label(item.mass.to_grams().to_string());
                            ui.label(item.value.to_string());
                        }
                    }

                    if ui.button("Edit").double_clicked() {
                        events.send(SpawnWindow::Record(window.records.clone(), record.id));
                    }
                });
            }

            if ui.button("Add").clicked() {
                let rec = Record {
                    id: RecordId(0),
                    name: "".to_owned(),
                    body: RecordBody::Item(ItemRecord {
                        mass: Mass::new(),
                        value: 0,
                    }),
                };

                window.records.insert(rec);
            }
        });
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Category {
    Items = 0,
}

impl Category {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Items => "Items",
        }
    }
}

#[derive(Clone, Debug)]
struct State {
    search: String,
    categories: [bool; 1],
}

#[derive(Clone, Debug, Component)]
pub struct RecordWindow {
    pub records: Records,
    pub id: RecordId,
}

fn render_record_windows(mut windows: Query<(&mut EguiContext, &mut RecordWindow)>) {
    for (mut ctx, state) in &mut windows {
        let mut record = state.records.get(state.id).unwrap();

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
                }
            }
        });

        if changed {
            state.records.put(record);
        }
    }
}
