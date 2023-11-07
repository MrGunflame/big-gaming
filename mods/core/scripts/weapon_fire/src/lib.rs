use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::inventory::{Inventory, InventoryId};
use game_wasm::world::{Entity, EntityBuilder, Object, RecordReference};
use shared::components::{AMMO, GUN_PROPERTIES};
use shared::{Ammo, GunProperties};

#[on_action]
fn on_action(entity: u64, invoker: u64) {
    let inventory = Inventory::new(EntityId::from_raw(invoker));

    let id = find_equipped_gun(&inventory).unwrap();

    let properties = inventory.component_get(id, GUN_PROPERTIES).unwrap();
    let mut ammo = inventory.component_get(id, AMMO).unwrap();

    let has_ammo = ammo.update(|ammo: &mut Ammo| ammo.try_decrement());

    if !has_ammo {
        return;
    }

    let properties: GunProperties = properties.read();

    inventory
        .component_insert(InventoryId(entity), AMMO, &ammo)
        .unwrap();

    build_projectile(EntityId::from_raw(invoker), properties.projectile);
}

fn build_projectile(invoker: EntityId, projectile: RecordReference) {
    let actor = Entity::get(invoker).unwrap();

    EntityBuilder::new(Object { id: projectile })
        .translation(actor.translation())
        .rotation(actor.rotation())
        .build()
        .spawn()
        .unwrap();
}

fn find_equipped_gun(inventory: &Inventory) -> Option<InventoryId> {
    for id in inventory.keys().unwrap() {
        let stack = inventory.get(id).unwrap();
        if stack.item.equipped && inventory.component_get(id, GUN_PROPERTIES).is_ok() {
            return Some(id);
        }
    }

    None
}
