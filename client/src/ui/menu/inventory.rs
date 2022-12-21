use bevy::prelude::With;
use bevy_egui::egui::{Area, Order, Pos2};
use common::archive::GameArchive;
use common::components::inventory::Inventory;

use crate::entities::player::PlayerCharacter;
use crate::ui::widgets::UiExt;
use crate::ui::Interface;

#[derive(Debug, Default)]
pub struct InventoryMenu {}

impl Interface for InventoryMenu {
    fn create(&mut self) {}

    fn render(&mut self, ctx: &bevy_egui::egui::Context, world: &mut bevy::prelude::World) {
        let inventory = world
            .query_filtered::<&Inventory, With<PlayerCharacter>>()
            .single(world);

        let archive = world.resource::<GameArchive>();

        Area::new("inventory")
            .fixed_pos(Pos2::new(0.0, 0.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.transparent_background(|ui| {
                    ui.heading("Inventory");

                    ui.label(format!("{} items", inventory.items()));

                    for stack in inventory {
                        let item = archive.item(stack.item.id).unwrap();
                        ui.label(format!("{:?} x{}", item.name, stack.quantity));
                    }
                });
            });
    }

    fn destroy(&mut self) {}
}
