use bevy_egui::egui::{Area, Pos2};

use crate::{Context, Widget, WidgetFlags};

#[derive(Default)]
pub struct GameMenu {}

impl Widget for GameMenu {
    fn flags(&self) -> WidgetFlags {
        WidgetFlags::CAPTURE_POINTER | WidgetFlags::CAPTURE_KEYS
    }

    fn create(&mut self) {}

    fn render(&mut self, ctx: &mut Context) {
        Area::new("gamemenu")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| {
                ui.label("Game menu");
            });
    }

    fn destroy(&mut self) {}
}
