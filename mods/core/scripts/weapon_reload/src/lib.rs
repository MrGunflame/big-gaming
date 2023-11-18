#![no_std]

use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::inventory::Inventory;
use shared::components::{AMMO, GUN_PROPERTIES};
use shared::{panic_handler, Ammo, GunProperties};

panic_handler!();

#[on_action]
fn on_action(invoker: EntityId) {
    let inventory = Inventory::new(invoker);

    for stack in inventory
        .iter()
        .unwrap()
        .filter(|stack| stack.item.equipped)
    {
        let Ok(properties) = stack.components().get(GUN_PROPERTIES) else {
            continue;
        };
        let properties: GunProperties = properties.read();

        let mut ammo = stack.components().entry(AMMO).or_default();
        ammo.write(Ammo(properties.magazine_capacity));

        stack.components().insert(AMMO, &ammo).unwrap();
    }
}
