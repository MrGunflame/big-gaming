use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::inventory::Inventory;
use game_wasm::world::{Entity, EntityBuilder, RecordReference};
use shared::components::{AMMO, GUN_PROPERTIES};
use shared::{Ammo, GunProperties};

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

        let mut ammo = stack
            .components()
            .entry(AMMO)
            .or_insert_with(|ammo| ammo.write(Ammo(properties.magazine_capacity)));

        let has_ammo = ammo.update(|ammo: &mut Ammo| ammo.try_decrement());

        if has_ammo {
            stack.components().insert(AMMO, &ammo).unwrap();
            build_projectile(invoker, properties.projectile);
        }
    }
}

fn build_projectile(invoker: EntityId, projectile: RecordReference) {
    let actor = Entity::get(invoker).unwrap();

    EntityBuilder::from_record(projectile)
        .translation(actor.translation())
        .rotation(actor.rotation())
        .spawn()
        .unwrap();
}
