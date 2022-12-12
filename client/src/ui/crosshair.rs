use bevy::prelude::ResMut;
use bevy_egui::egui::{
    Area, Color32, Order, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2, Widget,
};
use bevy_egui::EguiContext;

pub fn crosshair(mut egui: ResMut<EguiContext>) {
    Area::new("crosshair")
        .fixed_pos(Pos2::new(0.0, 0.0))
        .order(Order::Background)
        .show(egui.ctx_mut(), |ui| {
            ui.add(CrosshairWidget {});
        });
}

pub struct CrosshairWidget {}

impl Widget for CrosshairWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = ui.available_size() / 2.0;

        let rect = Rect {
            min: Pos2::new(size.x - 2.5, size.y - 2.5),
            max: Pos2::new(size.x + 2.5, size.y + 2.5),
        };

        let resp = ui.allocate_rect(
            rect,
            Sense {
                click: false,
                drag: false,
                focusable: false,
            },
        );

        if ui.is_rect_visible(resp.rect) {
            ui.painter()
                .rect(rect, Rounding::none(), Color32::BLUE, Stroke::none());
        }

        resp
    }
}
