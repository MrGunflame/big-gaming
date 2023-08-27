use bevy_egui::egui::CentralPanel;

use crate::{Widget, WidgetFlags};

#[derive(Debug, Default)]
pub struct Loading {}

impl Widget for Loading {
    fn name(&self) -> &'static str {
        "core::loading"
    }

    fn flags(&self) -> crate::WidgetFlags {
        WidgetFlags::IGNORE_CLOSE
    }

    fn render(&mut self, ctx: &mut crate::Context) {
        CentralPanel::default().show(ctx.ctx, |ui| {
            ui.label("Loading");
        });
    }
}