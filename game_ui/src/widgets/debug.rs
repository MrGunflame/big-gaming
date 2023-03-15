use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::prelude::{Camera3d, Transform, With};
use bevy_egui::egui::{Area, Pos2};
use game_common::components::player::HostPlayer;
use game_common::components::transform::PreviousTransform;
use game_common::math::RotationExt;
use game_common::world::source::StreamingSources;
use game_common::world::CellId;

use crate::{Context, Widget};

#[derive(Clone, Default)]
pub struct DebugInfo {}

impl Widget for DebugInfo {
    fn name(&self) -> &'static str {
        "core::debug"
    }

    fn render(&mut self, ctx: &mut Context) {
        let camera = ctx
            .world
            .query_filtered::<&Transform, With<Camera3d>>()
            .single(ctx.world)
            .clone();

        let (transform, prev) = ctx
            .world
            .query_filtered::<(&Transform, &PreviousTransform), With<HostPlayer>>()
            .single(ctx.world);

        let soures = ctx.world.resource::<StreamingSources>();

        let diags = ctx.world.resource::<Diagnostics>();
        let fps = diags
            .get(FrameTimeDiagnosticsPlugin::FPS)
            .unwrap()
            .value()
            .unwrap();
        let ft = diags
            .get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
            .unwrap()
            .value()
            .unwrap();

        Area::new("debug")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| {
                ui.label(format!("{:.2} FPS ({:.2} ms)", fps, ft));

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

                let x = transform.translation.x - prev.translation.x;
                let y = transform.translation.y - prev.translation.y;
                let z = transform.translation.z - prev.translation.z;
                ui.label(format!("DELTA X: {:.2} Y: {:.2} Z: {:.2}", x, y, z));

                let loaded = soures.loaded().count();
                let unloaded = soures.unloaded().count();
                ui.label(format!(
                    "CELL C={} L={} U={} D={}",
                    soures.len(),
                    loaded,
                    unloaded,
                    loaded + unloaded,
                ));

                // CAMERA
                let x = camera.translation.x;
                let y = camera.translation.y;
                let z = camera.translation.z;
                ui.label(format!("CAM ORIG X: {:.2} Y: {:.2} Z: {:.2}", x, y, z));

                let dir = camera.rotation.dir_vec();
                ui.label(format!(
                    "CAM DIR X: {:.2} Y: {:.2} Z: {:.2}",
                    dir.x, dir.y, dir.z
                ));
            });
    }
}
