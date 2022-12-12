use bevy::prelude::{Query, ResMut, With};
use bevy_egui::egui::{Area, Order, Pos2};
use bevy_egui::EguiContext;

use crate::components::ActorState;
use crate::entities::player::PlayerCharacter;

pub fn death(
    mut egui: ResMut<EguiContext>,
    mut players: Query<&mut ActorState, With<PlayerCharacter>>,
) {
    let mut state = players.single_mut();

    if *state != ActorState::DEAD {
        return;
    }

    Area::new("death")
        .fixed_pos(Pos2::new(0.0, 0.0))
        .order(Order::Foreground)
        .show(egui.ctx_mut(), |ui| {
            ui.label("You ded!");

            if ui.button("Respawn").clicked() {
                *state = ActorState::NORMAL;
            }
        });
}
