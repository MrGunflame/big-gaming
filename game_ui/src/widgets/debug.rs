use bevy::prelude::{Transform, With};
use bevy_egui::egui::{Area, Pos2};
use game_common::components::player::HostPlayer;
use game_common::math::RotationExt;
use game_common::world::CellId;

use crate::{Context, Widget, WidgetFlags};

#[derive(Clone, Default)]
pub struct DebugInfo {}

impl Widget for DebugInfo {
    fn name(&self) -> &'static str {
        "core::debug"
    }

    fn render(&mut self, ctx: &mut Context) {
        let transform = ctx
            .world
            .query_filtered::<&Transform, With<HostPlayer>>()
            .single(ctx.world);

        Area::new("debug")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| {
                let x = transform.translation.x;
                let y = transform.translation.y;
                let z = transform.translation.z;

                ui.label(format!("ORIG X: {:.2} Y: {:.2} Z: {:.2}", x, y, z));

                let dir = transform.rotation.dir_vec();
                ui.label(format!(
                    "DIR X: {:.2} Y: {:.2} Z: {:.2}",
                    dir.x, dir.y, dir.z
                ));

                let cell = CellId::new(x, y, z);
                let (x, y, z) = cell.as_parts();
                ui.label(format!("CELL {}:{}:{}", x as i32, y as i32, z as i32));
            });
    }
}
