use bevy::prelude::{Query, Res, ResMut, With};
use bevy_egui::egui::{Area, Order, Pos2};
use bevy_egui::EguiContext;
use common::components::inventory::Inventory;

use crate::entities::player::PlayerCharacter;
use crate::ui::interfaces::MENU_INVENTORY;
use crate::ui::widgets::UiExt;
use crate::ui::InterfaceState;

pub fn inventory(
    mut egui: ResMut<EguiContext>,
    state: Res<InterfaceState>,
    mut players: Query<&Inventory, With<PlayerCharacter>>,
) {
    // let state = unsafe {
    //     match state.get_mut::<_, State>(MENU_INVENTORY) {
    //         Some(state) => state,
    //         None => return,
    //     }
    // };

    let inventory = players.single_mut();

    Area::new("inventory")
        .fixed_pos(Pos2::new(0.0, 0.0))
        .order(Order::Foreground)
        .show(egui.ctx_mut(), |ui| {
            ui.transparent_background(|ui| {
                ui.heading("Inventory");
                ui.label(format!("{} items", inventory.items()));
                for stack in inventory {
                    ui.label(format!("{:?}", stack.item.id));
                }
            });
        });
}

enum State {}
