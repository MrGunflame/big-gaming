use game_wasm::action::Action;
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::dispatch_event_dynamic;
use game_wasm::inventory::{Inventory, InventoryId};
use game_wasm::world::{Entity, RecordReference};

use crate::components::{EQUIP, EQUIPPABLE, UNEQUIP};
use crate::{Camera, Equippable};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Equip(InventoryId);

impl Action for Equip {
    const ID: RecordReference = EQUIP;
}

pub fn on_equip(entity: EntityId, Equip(slot): Equip) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    let inventory = Inventory::new(camera.parent);

    let Ok(mut stack) = inventory.get(slot) else {
        return;
    };

    let Ok(equppable) = stack.components().get(EQUIPPABLE) else {
        return;
    };
    let equippable = Equippable::decode(equppable.reader()).unwrap();

    stack.equip(true).unwrap();

    dispatch_event_dynamic(
        equippable.on_equip,
        &ItemEquip {
            entity: camera.parent,
            slot,
        },
    );
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Unequip(InventoryId);

impl Action for Unequip {
    const ID: RecordReference = UNEQUIP;
}

pub fn on_uneqip(entity: EntityId, Unequip(slot): Unequip) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    let inventory = Inventory::new(camera.parent);

    let Ok(mut stack) = inventory.get(slot) else {
        return;
    };

    let Ok(equippable) = stack.components().get(EQUIPPABLE) else {
        return;
    };
    let equippable = Equippable::decode(equippable.reader()).unwrap();
    stack.equip(false).unwrap();

    dispatch_event_dynamic(
        equippable.on_uneqip,
        &ItemUnequip {
            entity: camera.parent,
            slot,
        },
    );
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ItemEquip {
    pub entity: EntityId,
    pub slot: InventoryId,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ItemUnequip {
    pub entity: EntityId,
    pub slot: InventoryId,
}
