use bevy_egui::egui::{Area, Pos2, TextEdit};

use crate::{Context, Widget, WidgetFlags};

#[derive(Debug, Default)]
pub struct Console {
    input_buffer: String,
}

impl Widget for Console {
    fn name(&self) -> &'static str {
        "core::console"
    }

    fn flags(&self) -> WidgetFlags {
        WidgetFlags::CAPTURE_KEYS | WidgetFlags::CAPTURE_POINTER
    }

    fn render(&mut self, ctx: &mut Context) {
        Area::new("console")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| {
                let input = TextEdit::singleline(&mut self.input_buffer).lock_focus(true);

                ui.add(input);
            });
    }
}
