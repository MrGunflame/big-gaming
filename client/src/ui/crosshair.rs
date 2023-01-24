use bevy::prelude::{Query, ResMut, With};
use bevy_egui::egui::{
    Area, Color32, Order, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2, Widget,
};
use bevy_egui::EguiContext;
use common::components::player::{FocusedEntity, HostPlayer};

use super::SenseExt;

pub fn crosshair(
    mut egui: ResMut<EguiContext>,
    mut focus: Query<&FocusedEntity, With<HostPlayer>>,
) {
    let mut focus = focus.single();

    let area = Area::new("crosshair")
        .fixed_pos(Pos2::new(0.0, 0.0))
        .order(Order::Background);

    match focus {
        FocusedEntity::Some {
            entity: _,
            distance: _,
        } => {
            area.show(egui.ctx_mut(), |ui| {
                ui.add(ItemCrosshair {
                    radius: 24.0,
                    width: 2.0,
                });
            });
        }
        FocusedEntity::None => {
            area.show(egui.ctx_mut(), |ui| {
                ui.add(CrosshairWidget {});
            });
        }
        _ => (),
    }
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

pub struct ItemCrosshair {
    radius: f32,
    width: f32,
}

impl Widget for ItemCrosshair {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = ui.available_size() / 2.0;

        let rect = Rect {
            min: Pos2::new(size.x - self.radius, size.y - self.radius),
            max: Pos2::new(size.x + self.radius, size.y + self.radius),
        };

        let resp = ui.allocate_rect(rect, Sense::none());

        if ui.is_rect_visible(resp.rect) {
            let painter = ui.painter();

            // TOP LEFT
            painter.line_segment(
                [
                    Pos2::new(rect.min.x, rect.min.y),
                    Pos2::new(rect.min.x, rect.min.y + self.radius / 2.0),
                ],
                Stroke::new(self.width, Color32::RED),
            );

            painter.line_segment(
                [
                    Pos2::new(rect.min.x, rect.min.y),
                    Pos2::new(rect.min.x + self.radius / 2.0, rect.min.y),
                ],
                Stroke::new(self.width, Color32::RED),
            );

            // BOTTOM LEFT
            painter.line_segment(
                [
                    Pos2::new(rect.min.x, rect.max.y),
                    Pos2::new(rect.min.x, rect.max.y - self.radius / 2.0),
                ],
                Stroke::new(self.width, Color32::RED),
            );

            painter.line_segment(
                [
                    Pos2::new(rect.min.x, rect.max.y),
                    Pos2::new(rect.min.x + self.radius / 2.0, rect.max.y),
                ],
                Stroke::new(self.width, Color32::RED),
            );

            // TOP RIGHT
            painter.line_segment(
                [
                    Pos2::new(rect.max.x, rect.min.y),
                    Pos2::new(rect.max.x, rect.min.y + self.radius / 2.0),
                ],
                Stroke::new(self.width, Color32::RED),
            );

            painter.line_segment(
                [
                    Pos2::new(rect.max.x, rect.min.y),
                    Pos2::new(rect.max.x - self.radius / 2.0, rect.min.y),
                ],
                Stroke::new(self.width, Color32::RED),
            );

            // BOTTOM RIGHT
            painter.line_segment(
                [
                    Pos2::new(rect.max.x, rect.max.y),
                    Pos2::new(rect.max.x, rect.max.y - self.radius / 2.0),
                ],
                Stroke::new(self.width, Color32::RED),
            );

            painter.line_segment(
                [
                    Pos2::new(rect.max.x, rect.max.y),
                    Pos2::new(rect.max.x - self.radius / 2.0, rect.max.y),
                ],
                Stroke::new(self.width, Color32::RED),
            );
        }

        resp
    }
}
