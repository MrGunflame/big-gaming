//! Module selectors

use bevy::prelude::{Component, Query, ResMut, With};
use bevy_egui::egui::{Align, CentralPanel, Layout};
use bevy_egui::EguiContext;

use super::Modules;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleWindowPlugin;

impl bevy::prelude::Plugin for ModuleWindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(render_modules);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct ModuleWindow;

fn render_modules(
    modules: ResMut<Modules>,
    mut windows: Query<&mut EguiContext, With<ModuleWindow>>,
) {
    for mut ctx in &mut windows {
        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.label("ID");
                ui.label("Name");
                ui.label("Dependencies");
            });

            for module in modules.modules.values() {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.label(module.module.id.to_string());
                    ui.label(module.module.name.clone());
                    ui.label(module.module.dependencies.len().to_string());
                });
            }

            if ui.button("New").clicked() {}
        });
    }
}
