use bevy::prelude::{EventWriter, Query, With};
use bevy::window::PrimaryWindow;
use bevy_egui::egui::{Align2, Area, Vec2};
use bevy_egui::EguiContext;

use crate::windows::SpawnWindow;

pub fn render_main_bar(
    mut windows: Query<&mut EguiContext, With<PrimaryWindow>>,
    mut events: EventWriter<SpawnWindow>,
) {
    let mut ctx = windows.single_mut();

    Area::new("main_bar")
        .anchor(Align2::LEFT_TOP, Vec2::splat(0.0))
        .show(ctx.get_mut(), |ui| {
            if ui.button("Modules").clicked() {
                events.send(SpawnWindow::Modules);
            }

            if ui.button("Templates").clicked() {
                events.send(SpawnWindow::Templates);
            }
        });
}
