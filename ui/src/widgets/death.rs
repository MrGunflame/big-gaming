use bevy::prelude::{Entity, With};
use bevy_egui::egui::{Area, Pos2};
use common::components::actor::{ActorFlag, ActorFlags, Spawn};
use common::components::player::HostPlayer;

use crate::{Context, Widget, WidgetFlags};

#[derive(Debug, Default)]
pub struct Death {}

impl Widget for Death {
    fn name(&self) -> &'static str {
        "core::death"
    }

    fn flags(&self) -> WidgetFlags {
        WidgetFlags::CAPTURE_POINTER | WidgetFlags::CAPTURE_KEYS | WidgetFlags::IGNORE_CLOSE
    }

    fn render(&mut self, ctx: &mut Context) {
        let (entity, flags) = ctx
            .world
            .query_filtered::<(Entity, &ActorFlags), With<HostPlayer>>()
            .single(ctx.world);

        if !flags.contains(ActorFlag::DEAD) {
            ctx.close();
            return;
        }

        Area::new("death")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label("You ded");

                    if ui.button("Respawn").clicked() {
                        ctx.world.entity_mut(entity).insert(Spawn);
                    }
                });
            });
    }
}
