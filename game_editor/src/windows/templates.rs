//! The template data editor.

use std::sync::Arc;

use bevy::prelude::{Component, Query};
use bevy_egui::egui::panel::Side;
use bevy_egui::egui::{Align, CentralPanel, Layout, SidePanel, TextEdit};
use bevy_egui::EguiContext;
use game_common::module::ModuleId;
use game_common::units::Mass;
use game_data::components::item::{Item, ItemId};
use game_data::DataBuffer;
use parking_lot::RwLock;
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplatesPlugin;

impl bevy::prelude::Plugin for TemplatesPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(render_window);
    }
}

#[derive(Clone, Debug, Component)]
pub struct TemplatesWindow {
    module: ModuleId,
    data: Arc<RwLock<DataBuffer>>,
    state: State,
}

impl TemplatesWindow {
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

fn render_window(mut windows: Query<(&mut EguiContext, &mut TemplatesWindow)>) {
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
