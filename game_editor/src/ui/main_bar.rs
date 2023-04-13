use bevy::prelude::{EventWriter, Query, Res, With};
use bevy::window::PrimaryWindow;
use bevy_egui::egui::{Align2, Area, Vec2};
use bevy_egui::EguiContext;

use crate::backend::{Handle, Task, WriteModule};
use crate::state::module::Modules;
use crate::state::record::Records;
use crate::windows::SpawnWindow;

pub fn render_main_bar(
    mut windows: Query<&mut EguiContext, With<PrimaryWindow>>,
    mut events: EventWriter<SpawnWindow>,
    modules: Res<Modules>,
    records: Res<Records>,
    handle: Res<Handle>,
) {
    let mut ctx = windows.single_mut();

    Area::new("main_bar")
        .anchor(Align2::LEFT_TOP, Vec2::splat(0.0))
        .show(ctx.get_mut(), |ui| {
            if ui.button("Modules").clicked() {
                events.send(SpawnWindow::Modules);
            }

            if ui.button("Records").clicked() {
                events.send(SpawnWindow::Templates);
            }

            if ui.button("Save").clicked() {
                for m in modules.iter() {
                    tracing::info!("saving module {} ({})", m.module.name, m.module.id);

                    handle.send(Task::WriteModule(WriteModule {
                        module: m.clone(),
                        records: records.clone(),
                    }));
                }
            }
        });
}
