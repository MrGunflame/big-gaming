use game_wasm::action::Action;
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::dispatch_event_dynamic;
use game_wasm::inventory::{Inventory, InventorySlotId, ItemStack};
use game_wasm::world::{Entity, RecordReference};

use crate::components::{EQUIP, UNEQUIP};
use crate::{Camera, Equippable};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Equip(InventorySlotId);

impl Action for Equip {
    const ID: RecordReference = EQUIP;
}

pub fn on_equip(entity: EntityId, Equip(slot): Equip) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    let actor = camera.parent;
    with_equippable(actor, slot, |stack, equippable| {
        stack.equipped = true;

        dispatch_event_dynamic(
            equippable.on_equip,
            &ItemEquip {
                entity: actor,
                slot,
            },
        );
    });
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Unequip(InventorySlotId);

impl Action for Unequip {
    const ID: RecordReference = UNEQUIP;
}

pub fn on_uneqip(entity: EntityId, Unequip(slot): Unequip) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    let actor = camera.parent;
    with_equippable(actor, slot, |stack, equippable| {
        stack.equipped = false;

        dispatch_event_dynamic(
            equippable.on_uneqip,
            &ItemUnequip {
                entity: camera.parent,
                slot,
            },
        );
    });
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ItemEquip {
    pub entity: EntityId,
    pub slot: InventorySlotId,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ItemUnequip {
    pub entity: EntityId,
    pub slot: InventorySlotId,
}

fn with_equippable<F>(entity: EntityId, slot: InventorySlotId, f: F)
where
    F: FnOnce(&mut ItemStack, Equippable),
{
    let entity = Entity::new(entity);

    let Ok(mut inventory) = entity.get::<Inventory>() else {
        return;
    };

    let Some(stack) = inventory.get_mut(slot) else {
        return;
    };

    let Ok(equippable) = stack.components.get::<Equippable>() else {
        return;
    };

    f(stack, equippable);

    entity.insert(inventory);
}
