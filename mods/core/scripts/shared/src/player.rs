use alloc::borrow::ToOwned;
use alloc::vec;
use game_wasm::action::Action;
use game_wasm::components::builtin::{
    Axis, Capsule, Collider, ColliderShape, Color, Cuboid, DirectionalLight, MeshInstance,
    RigidBody, RigidBodyKind, Transform, TriMesh,
};
use game_wasm::components::{Component, Components, RawComponent};
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::{Event, PlayerConnect};
use game_wasm::inventory::{Inventory, ItemStack};
use game_wasm::math::{Quat, Vec3};
use game_wasm::resource::{create_resource, ResourceId};
use game_wasm::world::{Entity, RecordReference};

use crate::actor::{spawn_actor, SpawnActor};
use crate::components::{
    EVENT_GUN_EQUIP, EVENT_GUN_UNEQUIP, PLAYER_RESPAWN, RESPAWN_POINT, TEST_WEAPON,
    TRANSFORM_CHANGED,
};
use crate::{
    assets, Camera, CharacterController, Equippable, GunProperties, Health, Humanoid,
    LookingDirection, MovementSpeed, PlayerCamera, Projectile, SpawnPoint,
};

pub fn spawn_player(_: EntityId, event: PlayerConnect) {
    let entity = spawn_actor(SpawnActor {
        mesh: MeshInstance {
            model: assets::RESOURCE_PERSON.into(),
        },
        collider: Collider {
            friction: 1.0,
            restitution: 1.0,
            shape: ColliderShape::Capsule(Capsule {
                axis: Axis::Y,
                half_height: 0.5,
                radius: 0.5,
            }),
        },
        mesh_offset: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
    });

    entity.insert(Transform {
        translation: Vec3::new(0.0, 10.0, 0.0),
        ..Default::default()
    });
    entity.insert(MovementSpeed(1.0));
    entity.insert(Humanoid);
    entity.insert(CharacterController);
    entity.insert(Health {
        value: 10,
        max: 100,
    });
    entity.insert(SpawnPoint {
        translation: Vec3::ZERO,
    });

    let mut inventory = Inventory::new();

    {
        let id = inventory.insert(ItemStack {
            item: TEST_WEAPON,
            equipped: false,
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

    entity.insert(inventory);

    entity.insert(PlayerCamera {
        camera: camera.id(),
        offset: Vec3::new(0.0, 0.8, 0.0),
        rotation: Quat::IDENTITY,
    });
    entity.insert(LookingDirection::default());

    entity.insert(RespawnPoint {
        position: Vec3::new(0.0, 10.0, 0.0),
    });

    event.player.set_active(camera.id());

    let pawn = Entity::spawn();
    pawn.insert(Transform::from_translation(Vec3::splat(10.0)));
    pawn.insert(MeshInstance {
        model: ResourceId::from(assets::RESOURCE_PERSON),
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

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RespawnPlayer {}

impl Action for RespawnPlayer {
    const ID: RecordReference = PLAYER_RESPAWN;
}

pub fn respawn_player(entity: EntityId, event: RespawnPlayer) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    let actor = Entity::new(camera.parent);

    let Ok(respawn_point) = actor.get::<RespawnPoint>() else {
        return;
    };
    let Ok(mut transform) = actor.get::<Transform>() else {
        return;
    };

    transform.translation = respawn_point.position;
    actor.insert(transform);
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct RespawnPoint {
    pub position: Vec3,
}

impl Component for RespawnPoint {
    const ID: RecordReference = RESPAWN_POINT;
}
