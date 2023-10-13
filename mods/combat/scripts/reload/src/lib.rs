use combat_shared::components::{AMMO, GUN_PROPERTIES};
use combat_shared::{Ammo, GunProperties};
use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::inventory::{Inventory, InventoryId};

#[on_action]
fn on_action(entity: u64, invoker: u64) {
    let inventory = Inventory::new(EntityId::from_raw(invoker));

    let properties = inventory
        .component_get(InventoryId(entity), GUN_PROPERTIES)
        .unwrap();

    let properties: GunProperties = properties.read();

    let mut ammo = inventory.component_get(InventoryId(entity), AMMO).unwrap();

    // Already all full capacity.
    if ammo.read::<Ammo>().0 == properties.magazine_capacity {
        return;
    }

    ammo.write::<Ammo>(Ammo(properties.magazine_capacity));

    inventory
        .component_insert(InventoryId(entity), AMMO, &ammo)
        .unwrap();
}
