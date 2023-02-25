use bevy::prelude::With;
use bevy_egui::egui::{Align2, Area, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use game_common::components::combat::Health as ActorHealth;
use game_common::components::player::HostPlayer;

use crate::{Context, Widget, WidgetFlags};

#[derive(Copy, Clone, Debug, Default)]
pub struct Health;

impl Widget for Health {
    fn name(&self) -> &'static str {
        "core::health"
    }

    fn flags(&self) -> WidgetFlags {
        WidgetFlags::IGNORE_CLOSE
    }

    fn render(&mut self, ctx: &mut Context) {
        let Ok(health) = ctx
            .world
            .query_filtered::<&ActorHealth, With<HostPlayer>>()
            .get_single(&ctx.world) else {
                return;
            };

        Area::new("health")
            .anchor(Align2::LEFT_BOTTOM, Vec2::new(5.0, -5.0))
            .show(ctx.ctx, |ui| {
                ui.add(HealthWidget {
                    width: 100.0,
                    height: 100.0,
                    health: *health,
                })
            });
    }
}

struct HealthWidget {
    width: f32,
    height: f32,
    health: ActorHealth,
}

impl bevy_egui::egui::Widget for HealthWidget {
    fn ui(self, ui: &mut bevy_egui::egui::Ui) -> bevy_egui::egui::Response {
        // The rectangle for the whole widget.
        let rect = Rect {
            min: Pos2::new(self.width, self.height),
            max: Pos2 {
                x: self.width,
                y: self.height,
            },
        };

        let (mut rect, resp) = ui.allocate_exact_size(
            Vec2::new(rect.min.x, rect.min.y),
            Sense {
                click: false,
                drag: false,
                focusable: false,
            },
        );

        if ui.is_rect_visible(resp.rect) {
            ui.painter()
                .rect(rect, Rounding::none(), Color32::RED, Stroke::NONE);

            // let mut rect = outer_rect;
            // let width = self.width * self.health.health as f32 / self.health.max_health as f32;
            // rect.max.x = width;
            let factor = self.health.health as f32 / self.health.max_health as f32;
            rect.max.x = rect.min.x + ((rect.max.x - rect.min.x) * factor);

            ui.painter()
                .rect(rect, Rounding::none(), Color32::GREEN, Stroke::NONE);

            ui.label(format!("{}/{}", self.health.health, self.health.max_health));
        }

        resp
    }
}
