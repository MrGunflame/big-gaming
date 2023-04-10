//! Module selectors

use bevy::prelude::{Component, Query, ResMut, With};
use bevy_egui::egui::{Align, CentralPanel, Layout};
use bevy_egui::EguiContext;
use egui_extras::{Column, TableBuilder};

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
            TableBuilder::new(ui)
                .columns(Column::remainder().resizable(true), 4)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Dependencies");
                    });
                    header.col(|ui| {
                        ui.heading("Writable");
                    });
                })
                .body(|mut body| {
                    for module in modules.modules.values() {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(module.module.id.to_string());
                            });
                            row.col(|ui| {
                                ui.label(module.module.name.clone());
                            });
                            row.col(|ui| {
                                ui.label(module.module.dependencies.len().to_string());
                            });
                            row.col(|ui| {
                                ui.label(module.capabilities.write().to_string());
                            });
                        });
                    }
                });
        });
    }
}
