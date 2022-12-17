use bevy::prelude::{Commands, Entity, Query, ResMut, With};
use bevy_egui::egui::{Area, Order, Pos2};
use bevy_egui::EguiContext;

use crate::components::ActorState;
use crate::entities::player::PlayerCharacter;
use crate::plugins::respawn::Respawn;
use crate::ui::widgets::UiExt;

pub fn death(
    mut commands: Commands,
    mut egui: ResMut<EguiContext>,
    mut players: Query<(Entity, &ActorState), With<PlayerCharacter>>,
) {
    let (entity, state) = players.single_mut();

    if *state != ActorState::DEAD {
        return;
    }

    Area::new("death")
        .fixed_pos(Pos2::new(0.0, 0.0))
        .order(Order::Foreground)
        .show(egui.ctx_mut(), |ui| {
            ui.transparent_background(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label("You ded, unlucky");

                    if ui.button("Respawn").clicked() {
                        commands.entity(entity).insert(Respawn::Normal);
                    }
                });
            });
        });
}
