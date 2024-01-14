#![no_std]

extern crate alloc;

use alloc::borrow::ToOwned;
use game_wasm::action::ActionBuffer;
use game_wasm::components::builtin::{MeshInstance, Transform};
use game_wasm::components::Decode;
use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::inventory::{Inventory, InventoryId};
use game_wasm::world::Entity;
use shared::components::EQUIPPABLE;
use shared::Equippable;

#[on_action]
pub fn on_action(entity: EntityId) {
    let slot_id: InventoryId = ActionBuffer::load().get().unwrap();

    let inventory = Inventory::new(entity);

    let Ok(mut stack) = inventory.get(slot_id) else {
        return;
    };

    let Ok(equppable) = stack.components().get(EQUIPPABLE) else {
        return;
    };
    let equippable = Equippable::decode(equppable.as_bytes()).unwrap();

    stack.equip(true).unwrap();

    let item = Entity::spawn();
    item.insert(Transform::default());
    item.insert(MeshInstance {
        path: "assets/AK.glb".to_owned(),
    });
}
