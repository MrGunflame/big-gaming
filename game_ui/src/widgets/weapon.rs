use bevy::prelude::With;
use bevy_egui::egui::{Align2, Area, Ui, Vec2};
use game_common::components::inventory::{Equipment, EquipmentSlot};
use game_common::components::items::Item;
use game_common::components::player::HostPlayer;

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

    fn render(&mut self, ctx: &mut Context) {
        let Ok(equipment) = ctx
            .world
            .query_filtered::<&Equipment, With<HostPlayer>>()
            .get_single(ctx.world) else {
                return;
            };

        let Some(item) = equipment.get(EquipmentSlot::MAIN_HAND) else {
            return;
        };

        Area::new("weapon")
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(0.0, 0.0))
            .show(ctx.ctx, |ui| render(ui, item));
    }
}

fn render(ui: &mut Ui, item: &Item) {
    if let Some(magazine) = &item.magazine {
        ui.label(magazine.count().to_string());
    }
}
