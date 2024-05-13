use core::f32::consts::PI;

use alloc::borrow::ToOwned;
use alloc::string::ToString;
use alloc::vec::Vec;
use game_wasm::action::Action;
use game_wasm::components::builtin::{MeshInstance, Transform};
use game_wasm::components::Component;
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::Event;
use game_wasm::hierarchy::Children;
use game_wasm::inventory::Inventory;
use game_wasm::math::{Quat, Ray, Vec3};
use game_wasm::physics::{cast_ray, QueryFilter};
use game_wasm::world::{Entity, RecordReference};

use crate::components::{
    EQUIPPED_ITEM, EVENT_GUN_EQUIP, EVENT_GUN_UNEQUIP, WEAPON_ATTACK, WEAPON_RELOAD,
};
use crate::inventory::{ItemEquip, ItemUnequip};
use crate::player::TransformChanged;
use crate::{Ammo, Camera, GunProperties, LookingDirection, ProjectileProperties};

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
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    let actor = Entity::new(camera.parent);
    let Ok(mut inventory) = actor.get::<Inventory>() else {
        return;
    };

    let Ok(looking_dir) = actor.get::<LookingDirection>() else {
        return;
    };
    let Ok(equipped_item) = actor.get::<EquippedItem>() else {
        return;
    };
    let mut exclude_entities = actor
        .get::<Children>()
        .map(|children| children.get().iter().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    exclude_entities.push(actor.id());

    let projectile_transform =
        project_camera_transform(looking_dir, equipped_item.offset, &exclude_entities);

    for stack in inventory
        .iter_mut()
        .map(|(_, stack)| stack)
        .filter(|stack| stack.equipped)
    {
        let Ok(properties) = stack.components.get::<GunProperties>() else {
            continue;
        };

        let mut ammo = stack
            .components
            .get::<Ammo>()
            .unwrap_or(Ammo(properties.magazine_capacity));

        let has_ammo = ammo.try_decrement();

        if has_ammo {
            stack.components.insert(ammo);

            build_projectile(
                actor.id(),
                projectile_transform,
                properties.projectile.id,
                properties.damage,
            );
        }
    }
}

fn project_camera_transform(
    camera: LookingDirection,
    item_offset: Vec3,
    exclude_entities: &[EntityId],
) -> Transform {
    const MAX_TOI: f32 = 100.0;

    let toi = match cast_ray(
        Ray {
            origin: camera.translation,
            direction: camera.rotation * -Vec3::Z,
        },
        MAX_TOI,
        QueryFilter { exclude_entities },
    ) {
        Some(hit) => hit.toi,
        None => MAX_TOI,
    };
    Transform::from_translation(camera.translation + camera.rotation * item_offset).looking_at(
        camera.translation + camera.rotation * Vec3::new(0.0, 0.0, -toi),
        Vec3::Y,
    )
}

fn build_projectile(
    owner: EntityId,
    transform: Transform,
    projectile: RecordReference,
    damage: f32,
) {
    let entity = Entity::spawn();
    entity.insert(transform);
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
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    let actor = Entity::new(camera.parent);

    let Ok(mut inventory) = actor.get::<Inventory>() else {
        return;
    };

    for stack in inventory
        .iter_mut()
        .map(|(_, stack)| stack)
        .filter(|stack| stack.equipped)
    {
        let Ok(properties) = stack.components.get::<GunProperties>() else {
            continue;
        };

        let ammo = Ammo(properties.magazine_capacity);
        stack.components.insert(ammo);
    }

    actor.insert(inventory);
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
        path: "assets/pistol.glb".to_owned(),
    });

    let owner = Entity::new(event.0.entity);
    owner.insert(EquippedItem {
        entity: entity.id(),
        offset: Vec3::new(0.2, -0.2, -0.5),
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
    pub offset: Vec3,
}

impl Component for EquippedItem {
    const ID: RecordReference = EQUIPPED_ITEM;
}

pub fn translate_equipped_items(_: EntityId, event: TransformChanged) {
    let entity = Entity::new(event.entity);

    let Ok(transform) = entity.get::<Transform>() else {
        return;
    };
    let Ok(equipped) = entity.get::<EquippedItem>() else {
        return;
    };
    let Ok(looking_dir) = entity.get::<LookingDirection>() else {
        return;
    };

    let item = Entity::new(equipped.entity);
    let mut item_transform = transform;
    item_transform.translation += item_transform.rotation * equipped.offset;
    item.insert(item_transform);

    item_transform.translation = looking_dir.translation + looking_dir.rotation * equipped.offset;

    //item_transform.translation = transform.translation + looking_dir.rotation * equipped.offset;
    // Yes somehow the default mesh is inverted around in the Y axis.
    item_transform.rotation = looking_dir.rotation * Quat::from_axis_angle(Vec3::Y, PI);

    item.insert(item_transform);
}
