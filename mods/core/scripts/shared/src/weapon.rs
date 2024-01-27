use alloc::borrow::ToOwned;
use alloc::string::ToString;
use game_wasm::action::Action;
use game_wasm::components::builtin::{MeshInstance, Transform};
use game_wasm::components::Component;
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::Event;
use game_wasm::inventory::Inventory;
use game_wasm::math::{Quat, Vec3};
use game_wasm::world::{Entity, RecordReference};

use crate::components::{
    AMMO, EQUIPPED_ITEM, EVENT_GUN_EQUIP, EVENT_GUN_UNEQUIP, GUN_PROPERTIES, WEAPON_ATTACK,
    WEAPON_RELOAD,
};
use crate::inventory::{ItemEquip, ItemUnequip};
use crate::{Ammo, GunProperties, ProjectileProperties};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct WeaponAttack;

impl Action for WeaponAttack {
    const ID: RecordReference = WEAPON_ATTACK;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct WeaponReload;

impl Action for WeaponReload {
    const ID: RecordReference = WEAPON_RELOAD;
}

pub fn weapon_attack(entity: EntityId, WeaponAttack: WeaponAttack) {
    let actor = Entity::new(entity);
    let inventory = Inventory::new(entity);

    let transform = actor.get::<Transform>().unwrap();

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
                actor.id(),
                translation,
                rotation,
                properties.projectile.id,
                properties.damage,
            );
        }
    }
}

fn build_projectile(
    owner: EntityId,
    translation: Vec3,
    rotation: Quat,
    projectile: RecordReference,
    damage: f32,
) {
    let entity = Entity::spawn();
    entity.insert(Transform {
        translation,
        rotation,
        scale: Vec3::splat(1.0),
    });
    entity.insert(ProjectileProperties {
        damage,
        owner,
        speed: 1.0,
    });
    entity.insert(MeshInstance {
        path: "assets/bullet.glb".to_string(),
    });
}

pub fn weapon_reload(entity: EntityId, WeaponReload: WeaponReload) {
    let inventory = Inventory::new(entity);

    for stack in inventory
        .iter()
        .unwrap()
        .filter(|stack| stack.item.equipped)
    {
        let Ok(properties) = stack.components().get(GUN_PROPERTIES) else {
            continue;
        };
        let properties = GunProperties::decode(properties.reader()).unwrap();

        let mut ammo = stack.components().entry(AMMO).or_default();
        ammo.write(Ammo(properties.magazine_capacity));

        stack.components().insert(AMMO, &ammo).unwrap();
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct GunEquip(ItemEquip);

impl Event for GunEquip {
    const ID: RecordReference = EVENT_GUN_EQUIP;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct GunUnequip(ItemUnequip);

impl Event for GunUnequip {
    const ID: RecordReference = EVENT_GUN_UNEQUIP;
}

pub fn gun_equip(_: EntityId, event: GunEquip) {
    let entity = Entity::spawn();
    entity.insert(Transform::default());
    entity.insert(MeshInstance {
        path: "assets/tyre.glb".to_owned(),
    });

    let owner = Entity::new(event.0.entity);
    owner.insert(EquippedItem {
        entity: entity.id(),
    });
}

pub fn gun_unequip(_: EntityId, event: GunUnequip) {
    let entity = Entity::new(event.0.entity);
    let equipped_item = entity.get::<EquippedItem>().unwrap();
    Entity::new(equipped_item.entity).despawn();
    entity.remove::<EquippedItem>();
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EquippedItem {
    pub entity: EntityId,
}

impl Component for EquippedItem {
    const ID: RecordReference = EQUIPPED_ITEM;
}
