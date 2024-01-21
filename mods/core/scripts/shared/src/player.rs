use alloc::borrow::ToOwned;
use game_wasm::components::builtin::{
    Collider, ColliderShape, Cuboid, MeshInstance, RigidBody, RigidBodyKind, Transform,
};
use game_wasm::components::RawComponent;
use game_wasm::entity::EntityId;
use game_wasm::events::PlayerConnect;
use game_wasm::inventory::{Inventory, Item, ItemStack};
use game_wasm::math::{Quat, Vec3};
use game_wasm::world::{Entity, RecordReference};

use crate::components::{
    EQUIPPABLE, EVENT_GUN_EQUIP, EVENT_GUN_UNEQUIP, GUN_PROPERTIES, TEST_WEAPON,
};
use crate::{
    CharacterController, Equippable, GunProperties, Health, Humanoid, MovementSpeed, Projectile,
    SpawnPoint,
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
        path: "assets/human.glb".to_owned(),
    });

    event.player.set_active(entity.id());

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
}
