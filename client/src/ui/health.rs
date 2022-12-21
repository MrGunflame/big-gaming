use bevy::prelude::{Query, ResMut, With};
use bevy_egui::egui::{
    Align2, Area, Color32, Order, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2, Widget,
};
use bevy_egui::EguiContext;
use common::components::combat::Health;

use crate::entities::player::PlayerCharacter;

pub fn health(mut egui: ResMut<EguiContext>, players: Query<&Health, With<PlayerCharacter>>) {
    let health = players.single();

    let health_percentage = if health.max_health == 0 {
        0.0
    } else {
        health.health as f32 / health.max_health as f32
    };

    Area::new("health")
        .anchor(Align2::LEFT_BOTTOM, Vec2::new(5.0, -5.0))
        .order(Order::Background)
        .show(egui.ctx_mut(), |ui| {
            ui.add(HealthWidget {
                width: 100.,
                height: 100.,
                health_percentage,
            });
        });
}

#[derive(Debug)]
pub struct HealthWidget {
    pub width: f32,
    pub height: f32,
    pub health_percentage: f32,
}

impl HealthWidget {
    pub const fn new() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            health_percentage: 0.0,
        }
    }
}

impl Widget for HealthWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        // The rectangle for the whole widget.
        let rect = Rect {
            min: Pos2::new(self.width, self.height),
            max: Pos2 {
                x: self.width,
                y: self.height,
            },
        };

        let (outer_rect, resp) = ui.allocate_exact_size(
            Vec2::new(rect.min.x, rect.min.y),
            Sense {
                click: false,
                drag: false,
                focusable: false,
            },
        );

        if ui.is_rect_visible(resp.rect) {
            ui.painter()
                .rect(outer_rect, Rounding::none(), Color32::RED, Stroke::none());

            let mut rect = outer_rect;
            let width = self.width * self.health_percentage;
            rect.max.x = width;

            ui.painter()
                .rect(rect, Rounding::none(), Color32::GREEN, Stroke::none());
        }

        resp
    }
}
