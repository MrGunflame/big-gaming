use game_wasm::components::builtin::{
    Collider, ColliderShape, Color, Cuboid, DirectionalLight, MeshInstance, RigidBody,
    RigidBodyKind, Transform,
};
use game_wasm::entity::EntityId;
use game_wasm::events::CellLoad;
use game_wasm::math::Vec3;
use game_wasm::resource::ResourceId;
use game_wasm::world::Entity;

use crate::assets;
use crate::weather::{sun_rotation, DateTime};

pub fn cell_load(_: EntityId, event: CellLoad) {
    let min = event.cell.min();
    let max = event.cell.max();

    if min.y != 0.0 {
        return;
    }

    let entity = Entity::spawn();
    entity.insert(Transform::from_translation(min));
    entity.insert(RigidBody {
        kind: RigidBodyKind::Fixed,
        linvel: Vec3::ZERO,
        angvel: Vec3::ZERO,
    });
    entity.insert(Collider {
        friction: 1.0,
        restitution: 1.0,
        shape: ColliderShape::Cuboid(Cuboid {
            hx: max.x - min.x,
            hy: 0.1,
            hz: max.z - min.z,
        }),
    });
    entity.insert(MeshInstance {
        model: ResourceId::from(assets::RESOURCE_FLOOR),
    });

    let sun = Entity::spawn();
    sun.insert(Transform {
        translation: min,
        rotation: sun_rotation(DateTime::from_secs(60 * 60 * 12)),
        ..Default::default()
    });
    sun.insert(DirectionalLight {
        color: Color::WHITE,
        illuminance: 100_000.0,
    });
}
