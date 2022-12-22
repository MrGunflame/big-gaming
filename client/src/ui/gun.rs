use bevy::prelude::{Query, ResMut, With};
use bevy_egui::egui::{Align2, Area, Order, Response, Ui, Vec2, Widget};
use bevy_egui::EguiContext;
use common::components::inventory::{Equipment, EquipmentSlot};

use crate::entities::player::PlayerCharacter;

pub fn gun(mut egui: ResMut<EguiContext>, players: Query<&Equipment, With<PlayerCharacter>>) {
    let equipment = players.single();

    let Some(item) = equipment.get(EquipmentSlot::MAIN_HAND) else {
        return;
    };

    let current = item.magazine.unwrap();

    Area::new("ammo")
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(0.0, 0.0))
        .order(Order::Background)
        .show(egui.ctx_mut(), |ui| {
            ui.add(Ammo { current });
        });
}

/// The current ammo widget.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ammo {
    current: u32,
}

impl Widget for Ammo {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.label("BUULETS");
        ui.label(self.current.to_string())
    }
}
