use alloc::borrow::ToOwned;
use game_wasm::components::builtin::{
    Collider, ColliderShape, Cuboid, MeshInstance, RigidBody, RigidBodyKind, Transform,
};
use game_wasm::entity::EntityId;
use game_wasm::events::PlayerConnect;
use game_wasm::math::Vec3;
use game_wasm::world::Entity;

use crate::{CharacterController, Health, Humanoid, MovementSpeed, SpawnPoint};

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
}
