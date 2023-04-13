use bevy::prelude::{Component, EventWriter, Query, ResMut};
use bevy_egui::egui::CentralPanel;
use bevy_egui::EguiContext;

use crate::backend::{Handle, Response};
use crate::state::module::Modules;
use crate::state::record::Records;

use super::SpawnWindow;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorWindowsPlugin;

impl bevy::prelude::Plugin for ErrorWindowsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(hanlde_backend_responses);
        app.add_system(render_error_windows);
    }
}

#[derive(Clone, Debug, Component)]
pub struct ErrorWindow {
    pub text: String,
}

fn hanlde_backend_responses(
    mut handle: ResMut<Handle>,
    mut modules: ResMut<Modules>,
    mut records: ResMut<Records>,
    mut events: EventWriter<SpawnWindow>,
) {
    while let Some(resp) = handle.recv() {
        match resp {
            Response::LoadModule(res) => match res {
                Ok((module, recs)) => {
                    for (_, rec) in recs.iter() {
                        records.insert(module.module.id, rec.clone());
                    }

                    modules.insert(module);
                }
                Err(err) => {
                    events.send(SpawnWindow::Error(format!(
                        "Failed to load module: {} ({:?})",
                        err, err,
                    )));
                }
            },
            Response::WriteModule(res) => match res {
                Ok(()) => (),
                Err(err) => {
                    events.send(SpawnWindow::Error(format!(
                        "Failed to write module: {}",
                        err
                    )));
                }
            },
        }
    }
}

fn render_error_windows(mut windows: Query<(&mut EguiContext, &mut ErrorWindow)>) {
    for (mut ctx, mut state) in &mut windows {
        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.label(&state.text);
        });
    }
}
