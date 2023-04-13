//! Module selectors

use std::path::PathBuf;

use bevy::prelude::{Commands, Component, Entity, EventWriter, Query, ResMut, With};
use bevy_egui::egui::{CentralPanel, TextEdit};
use bevy_egui::EguiContext;
use egui_extras::{Column, TableBuilder};
use game_common::module::{Module, ModuleId, Version};

use crate::backend::{Handle, Task};
use crate::state::capabilities::Capabilities;
use crate::state::module::{EditorModule, Modules};

use super::SpawnWindow;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleWindowPlugin;

impl bevy::prelude::Plugin for ModuleWindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(render_modules);

        app.add_system(render_create_module_windows);
        app.add_system(render_load_module_windows);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct ModuleWindow;

fn render_modules(
    modules: ResMut<Modules>,
    mut windows: Query<&mut EguiContext, With<ModuleWindow>>,
    mut events: EventWriter<SpawnWindow>,
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
                    for module in modules.iter() {
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

            if ui.button("Create new").clicked() {
                events.send(SpawnWindow::CreateModule);
            }

            if ui.button("Import").clicked() {
                events.send(SpawnWindow::ImportModule);
            }
        });
    }
}

#[derive(Clone, Debug, Component)]
pub struct CreateModuleWindow {
    id: ModuleId,
    name: String,
}

impl CreateModuleWindow {
    pub fn new() -> Self {
        Self {
            id: ModuleId::random(),
            name: String::new(),
        }
    }
}

fn render_create_module_windows(
    mut commands: Commands,
    mut modules: ResMut<Modules>,
    mut windows: Query<(Entity, &mut EguiContext, &mut CreateModuleWindow)>,
) {
    for (entity, mut ctx, mut state) in &mut windows {
        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.heading("Create Module");

            ui.label("ID");
            ui.label(state.id.to_string());

            ui.label("Name");
            ui.add(TextEdit::singleline(&mut state.name));

            if ui.button("OK").clicked() {
                let module = EditorModule {
                    module: Module {
                        id: state.id,
                        name: std::mem::take(&mut state.name),
                        version: Version,
                        dependencies: vec![],
                    },
                    path: PathBuf::from(format!("./{}", state.id)),
                    capabilities: Capabilities::READ | Capabilities::WRITE,
                };

                modules.insert(module);

                commands.entity(entity).despawn();
            }
        });
    }
}

#[derive(Clone, Debug, Default, Component)]
pub struct LoadModuleWindow {
    path: String,
    writable: bool,
}

fn render_load_module_windows(
    mut commands: Commands,
    mut windows: Query<(Entity, &mut EguiContext, &mut LoadModuleWindow)>,
    mut handle: ResMut<Handle>,
) {
    for (entity, mut ctx, mut state) in &mut windows {
        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.heading("Load module");

            ui.label("Local Path");
            ui.add(TextEdit::singleline(&mut state.path));

            ui.checkbox(&mut state.writable, "Writable");

            if ui.button("Ok").clicked() {
                handle.send(Task::ReadModule(state.path.clone().into()));
                commands.entity(entity).despawn();
            }
        });
    }
}
