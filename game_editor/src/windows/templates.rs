//! The template data editor.

use std::sync::Arc;

use bevy::prelude::{Component, Query, With};
use bevy_egui::egui::{Align, CentralPanel, Layout};
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
    pub module: ModuleId,
    pub data: Arc<RwLock<DataBuffer>>,
}

fn render_window(mut windows: Query<(&mut EguiContext, &TemplatesWindow)>) {
    for (mut ctx, window) in &mut windows {
        let data = window.data.read();

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
enum Categories {
    Items,
}

struct State {}
