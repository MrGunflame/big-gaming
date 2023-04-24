use bevy_egui::egui::{Align2, Area, Ui, Vec2};
use game_common::components::inventory::{Equipment, EquipmentSlot};
use game_common::components::items::Item;

use crate::{Context, Widget, WidgetFlags};

#[derive(Copy, Clone, Debug, Default)]
pub struct Weapon;

impl Widget for Weapon {
    fn name(&self) -> &'static str {
        "core::weapon"
    }

    fn flags(&self) -> WidgetFlags {
        WidgetFlags::IGNORE_CLOSE
    }

    fn render(&mut self, ctx: &mut Context) {}
}

fn render(ui: &mut Ui, item: &Item) {
    // if let Some(magazine) = &item.magazine {
    //     ui.label(magazine.count().to_string());
    // }
}
