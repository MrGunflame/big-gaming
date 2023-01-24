use bevy::prelude::{Query, ResMut, With};
use bevy_egui::egui::{
    Align2, Area, Color32, Order, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2, Widget,
};
use bevy_egui::EguiContext;
use common::components::combat::Health;
use common::components::player::HostPlayer;

pub fn health(mut egui: ResMut<EguiContext>, players: Query<&Health, With<HostPlayer>>) {
    let health = players.single();

    Area::new("health")
        .anchor(Align2::LEFT_BOTTOM, Vec2::new(5.0, -5.0))
        .order(Order::Background)
        .show(egui.ctx_mut(), |ui| {
            ui.add(HealthWidget {
                width: 100.,
                height: 100.,
                health: *health,
            });
        });
}

#[derive(Debug)]
pub struct HealthWidget {
    pub width: f32,
    pub height: f32,
    pub health: Health,
}

impl HealthWidget {
    pub const fn new() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            health: Health::new(0),
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
                .rect(rect, Rounding::none(), Color32::RED, Stroke::none());

            // let mut rect = outer_rect;
            // let width = self.width * self.health.health as f32 / self.health.max_health as f32;
            // rect.max.x = width;
            let factor = self.health.health as f32 / self.health.max_health as f32;
            rect.max.x = rect.min.x + ((rect.max.x - rect.min.x) * factor);

            ui.painter()
                .rect(rect, Rounding::none(), Color32::GREEN, Stroke::none());

            ui.label(format!("{}/{}", self.health.health, self.health.max_health));
        }

        resp
    }
}
