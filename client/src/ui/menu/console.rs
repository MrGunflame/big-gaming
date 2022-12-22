use bevy_egui::egui::{Area, Order, Pos2};

use super::super::{Interface, InterfaceKind};

#[derive(Debug, Default)]
pub struct Console {
    input_line: String,
}

impl Interface for Console {
    fn kind(&self) -> InterfaceKind {
        InterfaceKind::Interface
    }

    fn create(&mut self) {}

    fn render(&mut self, ctx: &bevy_egui::egui::Context, world: &mut bevy::prelude::World) {
        Area::new("console")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Hello world");
            });
    }

    fn destroy(&mut self) {}
}
