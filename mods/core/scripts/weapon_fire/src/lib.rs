#![no_std]

extern crate alloc;

use alloc::string::ToString;
use game_wasm::components::builtin::{MeshInstance, Transform};
use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::inventory::Inventory;
use game_wasm::math::Quat;
use game_wasm::world::{Entity, RecordReference};
use shared::components::{AMMO, GUN_PROPERTIES};
use shared::{panic_handler, Ammo, GunProperties, ProjectileProperties, Vec3};

panic_handler!();

#[on_action]
fn on_action(invoker: EntityId) {
    let actor = Entity::new(invoker);
    let inventory = Inventory::new(invoker);

    let transform = actor.get::<Transform>();

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

            let translation =
                transform.translation + Vec3::from_array(properties.projectile.translation);
            let rotation = transform.rotation * Quat::from_array(properties.projectile.rotation);

            build_projectile(
                translation,
                rotation,
                properties.projectile.id,
                properties.damage,
            );
        }
    }
}

fn build_projectile(translation: Vec3, rotation: Quat, projectile: RecordReference, damage: f32) {
    let entity = Entity::spawn();
    entity.insert(Transform {
        translation,
        rotation,
        scale: Vec3::splat(1.0),
    });
    entity.insert(ProjectileProperties { damage });
    entity.insert(MeshInstance {
        path: "assets/box.glb".to_string(),
    });
}
