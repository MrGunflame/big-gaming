use bevy::prelude::{Commands, Entity, Query, ResMut, With};
use bevy_egui::egui::{Area, Context, Order, Pos2};
use bevy_egui::EguiContext;

use crate::components::ActorState;
use crate::entities::player::PlayerCharacter;
use crate::plugins::respawn::Respawn;
use crate::ui::widgets::UiExt;
use crate::ui::{Focus, Interface};

pub struct Death {}

impl Interface for Death {
    fn create(&mut self) {}

    fn render(&mut self, ctx: &Context, world: &mut bevy::prelude::World) {
        let (entity, state) = world
            .query_filtered::<(Entity, &ActorState), With<PlayerCharacter>>()
            .single(world);

        if *state != ActorState::DEAD {
            return;
        }

        Area::new("death")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.transparent_background(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("You ded, unlucky");

                        if ui.button("Respawn").clicked() {
                            world.entity_mut(entity).insert(Respawn::Normal);
                        }
                    });
                });
            });
    }

    fn destroy(&mut self) {}
}
