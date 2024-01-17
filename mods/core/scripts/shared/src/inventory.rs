use alloc::borrow::ToOwned;
use game_wasm::action::Action;
use game_wasm::components::builtin::{MeshInstance, Transform};
use game_wasm::components::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::inventory::{Inventory, InventoryId};
use game_wasm::world::{Entity, RecordReference};

use crate::components::{EQUIP, EQUIPPABLE};
use crate::Equippable;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct Equip(InventoryId);

impl Action for Equip {
    const ID: RecordReference = EQUIP;
}

pub fn on_equip(entity: EntityId, Equip(slot): Equip) {
    let inventory = Inventory::new(entity);

    let Ok(mut stack) = inventory.get(slot) else {
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
        path: "assets/human.glb".to_owned(),
    });
}
