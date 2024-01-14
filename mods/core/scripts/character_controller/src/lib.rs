#![no_std]

use game_wasm::components::builtin::{Collider, ColliderShape, RigidBody, Transform};
use game_wasm::entity::EntityId;
use game_wasm::events::on_update;
use game_wasm::math::Real;
use game_wasm::physics::{cast_shape, QueryFilter};
use game_wasm::world::Entity;
use shared::Vec3;

const G: f32 = -9.81;
const DT: f32 = 1.0 / 60.0;

#[on_update]
fn on_update(entity: EntityId) {
    let entity = Entity::new(entity);

    let mut transform = entity.get::<Transform>().unwrap();
    let mut rigid_body = entity.get::<RigidBody>().unwrap();
    let collider = entity.get::<Collider>().unwrap();

    apply_gravity(
        entity.id(),
        &mut transform,
        &mut rigid_body.linvel,
        &collider.shape,
    );

    entity.insert(transform);
    entity.insert(rigid_body);
}

fn apply_gravity(
    entity: EntityId,
    transform: &mut Transform,
    linvel: &mut Vec3,
    shape: &ColliderShape,
) {
    let gravity = Vec3::new(0.0, -1.0, 0.0);
    let v0 = linvel.y;

    let max_toi = v0 * DT + 0.5 * G * DT * DT;

    let filter = QueryFilter {
        exclude_entities: &[entity],
    };

    let (distance, hit) = match cast_shape(
        transform.translation,
        transform.rotation,
        gravity,
        shape,
        // Note that `cast_shape` uses `gravity * max_toi` as the final max toi,
        // but `gravity` is already negative and `max_toi` must not be negative
        // for the shape cast to return useful results.
        // This also means that the returned `toi` and hit must be negated.
        max_toi.abs(),
        filter,
    ) {
        Some(hit) => (-hit.toi, true),
        None => (max_toi, false),
    };

    transform.translation.y += distance;
    if hit {
        linvel.y = 0.0;
    } else {
        linvel.y += G * DT;
    }
}
