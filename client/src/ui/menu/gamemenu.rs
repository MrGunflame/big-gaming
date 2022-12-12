use bevy::prelude::ResMut;
use bevy_egui::egui::{Area, Button, Order, Pos2};
use bevy_egui::EguiContext;

pub fn gamemenu(mut egui: ResMut<EguiContext>) {
    Area::new("gamemenu")
        .fixed_pos(Pos2::new(0.0, 0.0))
        .order(Order::Foreground)
        .show(egui.ctx_mut(), |ui| {
            ui.label("x");

            // if ui.button("Exit Game").clicked() {
            //     dbg!("exit game");
            // }
        });
}

pub struct GameMenu {}
