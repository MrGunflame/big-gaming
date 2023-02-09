use bevy::prelude::{Query, ResMut, With};
use bevy_egui::egui::{Align2, Area, Order, Response, Ui, Vec2, Widget};
use bevy_egui::EguiContext;
use game_common::components::inventory::{Equipment, EquipmentSlot};
use game_common::components::player::HostPlayer;

pub fn gun(mut egui: ResMut<EguiContext>, players: Query<&Equipment, With<HostPlayer>>) {
    let equipment = players.single();

    let Some(item) = equipment.get(EquipmentSlot::MAIN_HAND) else {
        return;
    };

    Area::new("ammo")
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(0.0, 0.0))
        .order(Order::Background)
        .show(egui.ctx_mut(), |ui| {});
}

/// The current ammo widget.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ammo {
    current: u16,
}

impl Widget for Ammo {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.label("BUULETS");
        ui.label(self.current.to_string())
    }
}
