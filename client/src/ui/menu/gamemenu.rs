use bevy::prelude::{Res, ResMut};
use bevy_egui::egui::{Area, Button, Color32, Order, Pos2};
use bevy_egui::EguiContext;

use crate::ui::interfaces::MENU_GAME;
use crate::ui::widgets::UiExt;
use crate::ui::{InterfaceId, InterfaceState};

pub fn gamemenu(mut egui: ResMut<EguiContext>, state: Res<InterfaceState>) {
    if !state.is_open(MENU_GAME) {
        return;
    }

    Area::new("gamemenu")
        .fixed_pos(Pos2::new(0.0, 0.0))
        .order(Order::Foreground)
        .show(egui.ctx_mut(), |ui| {
            ui.transparent_background(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label("Menu");

                    if ui.button("Exit Game").clicked() {
                        std::process::exit(0);
                    }

                    ui.label("texxt");
                });
            });
        });
}
