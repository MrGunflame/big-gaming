use alloc::borrow::ToOwned;
use game_wasm::components::builtin::{
    Collider, ColliderShape, Cuboid, MeshInstance, RigidBody, RigidBodyKind, Transform,
};
use game_wasm::components::RawComponent;
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::{Event, PlayerConnect};
use game_wasm::inventory::{Inventory, Item, ItemStack};
use game_wasm::math::{Quat, Vec3};
use game_wasm::world::{Entity, RecordReference};

use crate::components::{
    EQUIPPABLE, EVENT_GUN_EQUIP, EVENT_GUN_UNEQUIP, GUN_PROPERTIES, TEST_WEAPON, TRANSFORM_CHANGED,
};
use crate::{
    Camera, CharacterController, Equippable, GunProperties, Health, Humanoid, MovementSpeed,
    PlayerCamera, Projectile, SpawnPoint,
};

pub fn spawn_player(_: EntityId, event: PlayerConnect) {
    let entity = Entity::spawn();
    entity.insert(Transform {
        translation: Vec3::new(0.0, 10.0, 0.0),
        ..Default::default()
    });
    entity.insert(RigidBody {
        kind: RigidBodyKind::Fixed,
        linvel: Vec3::ZERO,
        angvel: Vec3::ZERO,
    });
    entity.insert(Collider {
        friction: 1.0,
        restitution: 1.0,
        shape: ColliderShape::Cuboid(Cuboid {
            hx: 1.0,
            hy: 1.0,
            hz: 1.0,
        }),
    });
    entity.insert(MovementSpeed(1.0));
    entity.insert(Humanoid);
    entity.insert(CharacterController);
    entity.insert(Health { value: 1, max: 100 });
    entity.insert(SpawnPoint {
        translation: Vec3::ZERO,
    });
    entity.insert(MeshInstance {
        path: "assets/person2.glb".to_owned(),
    });

    let inventory = Inventory::new(entity.id());
    let id = inventory
        .insert(ItemStack {
            item: Item {
                id: TEST_WEAPON,
                equipped: true,
                hidden: false,
            },
            quantity: 1,
        })
        .unwrap();

    let mut buf = RawComponent::default();
    buf.write(GunProperties {
        damage: 1.0,
        cooldown: 1.0,
        magazine_capacity: 30,
        projectile: Projectile {
            id: RecordReference::STUB,
            translation: Vec3::ZERO.to_array(),
            rotation: Quat::IDENTITY.to_array(),
        },
    });

    inventory
        .component_insert(id, GUN_PROPERTIES, &buf)
        .unwrap();

    let mut buf = RawComponent::default();
    buf.write(Equippable {
        on_equip: EVENT_GUN_EQUIP,
        on_uneqip: EVENT_GUN_UNEQUIP,
    });

    inventory.component_insert(id, EQUIPPABLE, &buf).unwrap();

    let camera = Entity::spawn();
    camera.insert(Transform::default());
    camera.insert(Camera {
        parent: entity.id(),
    });
    // Apply actions to the player camera controller so we can forward them
    // to the player actor.
    camera.insert(Humanoid);

    entity.insert(PlayerCamera {
        camera: camera.id(),
        offset: Vec3::new(0.0, 1.8, 0.0),
    });

    event.player.set_active(camera.id());
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct TransformChanged {
    pub entity: EntityId,
}

impl Event for TransformChanged {
    const ID: RecordReference = TRANSFORM_CHANGED;
}

pub fn update_camera_transform(_: EntityId, event: TransformChanged) {
    let entity = Entity::new(event.entity);
    let Ok(mut transform) = entity.get::<Transform>() else {
        return;
    };
    let Ok(camera) = entity.get::<PlayerCamera>() else {
        return;
    };

    transform.translation += camera.offset;
    let camera = Entity::new(camera.camera);
    camera.insert(transform);
}
