//! The template data editor.

use bevy::prelude::{Component, Query, With};
use bevy_egui::egui::{Align, CentralPanel, Layout};
use bevy_egui::EguiContext;
use game_common::units::Mass;
use game_data::components::item::{Item, ItemId};
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplatesPlugin;

impl bevy::prelude::Plugin for TemplatesPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(render_window);
    }
}

#[derive(Copy, Clone, Debug, Component)]
pub struct TemplatesWindow;

fn render_window(mut windows: Query<&mut EguiContext, With<TemplatesWindow>>) {
    let Ok(mut ctx) = windows.get_single_mut() else { return; };

    let items = vec![
        Item {
            id: ItemId(0),
            name: "test".to_owned(),
            mass: Mass::from_grams(10),
        },
        Item {
            id: ItemId(1),
            name: "x".to_owned(),
            mass: Mass::from_grams(11),
        },
    ];

    CentralPanel::default().show(ctx.get_mut(), |ui| {
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.label("ID");
            ui.label("Name");
            ui.label("Mass");
        });

        for item in items {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.label(format!("{:?}", item.id));
                ui.label(item.name);
                ui.label(item.mass.to_grams().to_string());
            });
        }
    });
}
