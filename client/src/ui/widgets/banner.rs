use bevy_egui::egui::{
    Align2, Color32, FontFamily, FontId, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2,
    Widget,
};

use crate::ui::SenseExt;

#[derive(Copy, Clone, Debug)]
pub struct Banner<'a> {
    pub label: &'a str,
    pub height: f32,
}

impl<'a> Widget for Banner<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = ui.available_size();

        let resp = ui.allocate_response(Vec2::new(size.x, self.height), Sense::none());

        let painter = ui.painter();
        painter.rect(resp.rect, Rounding::none(), Color32::BLUE, Stroke::none());
        painter.text(
            Pos2::new(resp.rect.min.x, resp.rect.min.y),
            Align2::LEFT_TOP,
            self.label,
            FontId::proportional(self.height),
            Color32::WHITE,
        );

        resp
    }
}
