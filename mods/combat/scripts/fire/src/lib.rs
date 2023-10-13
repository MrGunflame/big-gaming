use combat_shared::components::{AMMO, GUN_PROPERTIES};
use combat_shared::{Ammo, GunProperties};
use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::inventory::{Inventory, InventoryId};
use game_wasm::world::{Entity, EntityBuilder, Object, RecordReference};

#[on_action]
fn on_action(entity: u64, invoker: u64) {
    let inventory = Inventory::new(EntityId::from_raw(invoker));

    let properties = inventory
        .component_get(InventoryId(entity), GUN_PROPERTIES)
        .unwrap();
    let mut ammo = inventory.component_get(InventoryId(entity), AMMO).unwrap();

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
