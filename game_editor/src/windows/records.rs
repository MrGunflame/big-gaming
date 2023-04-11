//! The template data editor.

use std::fmt::{self, Display, Formatter, LowerHex};
use std::sync::Arc;

use bevy::prelude::{Component, EventWriter, Query};
use bevy_egui::egui::panel::Side;
use bevy_egui::egui::{Align, CentralPanel, Layout, SidePanel, TextEdit};
use bevy_egui::EguiContext;
use game_common::module::ModuleId;
use game_common::units::Mass;
use game_data::components::item::{Item, ItemId};
use game_data::DataBuffer;
use parking_lot::RwLock;

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
    data: Arc<RwLock<DataBuffer>>,
    state: State,
}

impl RecordsWindow {
    pub fn new(module: ModuleId, data: Arc<RwLock<DataBuffer>>) -> Self {
        Self {
            module,
            data,
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

        let data = window.data.read();

        SidePanel::new(Side::Left, "form_selector").show(ctx.get_mut(), |ui| {
            ui.add(TextEdit::singleline(&mut window.state.search).hint_text("Search"));

            for category in &[Category::Items] {
                ui.label(category.as_str());

                for item in &data.items {
                    ui.label(format!("{} [{}]", item.name, item.id));
                }
            }
        });

        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.label("ID");
                ui.label("Name");
                ui.label("Mass");
            });

            for item in &data.items {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.label(format!("{:?}", item.id));
                    ui.label(item.name.clone());
                    ui.label(item.mass.to_grams().to_string());
                });
            }

            if ui.button("Add").clicked() {
                let item = Item {
                    id: ItemId(0),
                    name: "".to_owned(),
                    mass: Mass::from_grams(0),
                };

                drop(data);
                let mut data = window.data.write();
                data.items.push(item);
            }

            if ui.button("Rec").clicked() {
                events.send(SpawnWindow::Record);
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
    pub module: ModuleId,
    pub record: Record,
}

#[derive(Clone, Debug)]
pub struct Record {
    pub id: RecordId,
    pub name: String,
    pub body: RecordBody,
}

#[derive(Clone, Debug)]
pub enum RecordBody {
    Item(ItemRecord),
}

#[derive(Clone, Debug)]
pub struct ItemRecord {
    pub mass: Mass,
    // TODO: Add separate Value type.
    pub value: u64,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RecordId(pub u32);

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

fn render_record_windows(mut windows: Query<(&mut EguiContext, &mut RecordWindow)>) {
    for (mut ctx, mut state) in &mut windows {
        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.heading("Metadata");

            ui.label("Record ID");
            ui.add_enabled(
                false,
                TextEdit::singleline(&mut state.record.id.to_string()).interactive(false),
            );

            ui.label("Name");
            ui.add(TextEdit::singleline(&mut state.record.name));

            match &mut state.record.body {
                RecordBody::Item(item) => {
                    ui.heading("Item");

                    ui.label("Mass (g)");

                    let mut mass = item.mass.to_grams().to_string();
                    if ui.add(TextEdit::singleline(&mut mass)).changed() {
                        let val = mass.parse::<u32>().unwrap_or_default();
                        item.mass = Mass::from_grams(val);
                    }

                    ui.label("Value");

                    let mut value = item.value.to_string();
                    if ui.add(TextEdit::singleline(&mut value)).changed() {
                        let val = value.parse::<u64>().unwrap_or_default();
                        item.value = val;
                    }
                }
            }
        });
    }
}
