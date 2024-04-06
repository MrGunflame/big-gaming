use alloc::borrow::ToOwned;
use game_wasm::components::builtin::{
    Collider, ColliderShape, Color, Cuboid, DirectionalLight, MeshInstance, RigidBody,
    RigidBodyKind, Transform,
};
use game_wasm::components::{Components, RawComponent};
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::{Event, PlayerConnect};
use game_wasm::inventory::{Inventory, ItemStack};
use game_wasm::math::{Quat, Vec3};
use game_wasm::world::{Entity, RecordReference};

use crate::components::{EVENT_GUN_EQUIP, EVENT_GUN_UNEQUIP, TEST_WEAPON, TRANSFORM_CHANGED};
use crate::{
    Camera, CharacterController, Equippable, GunProperties, Health, Humanoid, LookingDirection,
    MovementSpeed, PlayerCamera, Projectile, SpawnPoint,
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

    let mut inventory = Inventory::new();

    {
        let id = inventory.insert(ItemStack {
            item: TEST_WEAPON,
            equipped: true,
            hidden: false,
            quantity: 1,
            components: Components::default(),
        });

        let slot = inventory.get_mut(id).unwrap();

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

        slot.components.insert(GunProperties {
            damage: 1.0,
            cooldown: 1.0,
            magazine_capacity: 30,
            projectile: Projectile {
                id: RecordReference::STUB,
                translation: Vec3::ZERO.to_array(),
                rotation: Quat::IDENTITY.to_array(),
            },
        });
        slot.components.insert(Equippable {
            on_equip: EVENT_GUN_EQUIP,
            on_uneqip: EVENT_GUN_UNEQUIP,
        });
    }

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
        rotation: Quat::IDENTITY,
    });
    entity.insert(LookingDirection::default());

    event.player.set_active(camera.id());

    let dir_light = Entity::spawn();
    dir_light.insert(
        Transform {
            translation: Vec3::splat(100.0),
            ..Default::default()
        }
        .looking_at(Vec3::splat(0.0), Vec3::Y),
    );
    dir_light.insert(DirectionalLight {
        color: Color::WHITE,
        illuminance: 100_000.0,
    });

    let pawn = Entity::spawn();
    pawn.insert(Transform::from_translation(Vec3::splat(5.0)));
    pawn.insert(MeshInstance {
        path: "assets/person2.glb".to_owned(),
    });
    pawn.insert(RigidBody {
        kind: RigidBodyKind::Kinematic,
        linvel: Vec3::ZERO,
        angvel: Vec3::ZERO,
    });
    pawn.insert(Collider {
        friction: 1.0,
        restitution: 1.0,
        shape: ColliderShape::Cuboid(Cuboid {
            hx: 1.0,
            hy: 1.0,
            hz: 1.0,
        }),
    });
    pawn.insert(CharacterController);
    pawn.insert(Health {
        value: 100,
        max: 100,
    });
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
    transform.rotation = camera.rotation;
    let camera = Entity::new(camera.camera);
    camera.insert(transform);

    entity.insert(LookingDirection {
        translation: transform.translation,
        rotation: transform.rotation,
    });
}
