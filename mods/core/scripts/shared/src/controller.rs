use game_wasm::components::builtin::{ColliderShape, Transform};
use game_wasm::entity::EntityId;
use game_wasm::events::dispatch_event;
use game_wasm::math::Real;
use game_wasm::math::Vec3;
use game_wasm::physics::{cast_shape, QueryFilter};

pub const OFFSET: f32 = 0.1;

pub fn move_shape(
    entity: EntityId,
    transform: &mut Transform,
    direction: Vec3,
    shape: &ColliderShape,
) {
    let max_toi = direction.length();
    let filter = QueryFilter {
        exclude_entities: &[entity],
    };

    let max_distance = max_toi;

    let distance = match cast_shape(
        transform.translation,
        transform.rotation,
        direction,
        shape,
        max_toi + OFFSET,
        filter,
    ) {
        Some(hit) => {
            game_wasm::error!("{:?} {:?}", max_distance, hit);
            let allowed_distance = f32::max(hit.toi.abs() - OFFSET, 0.0);
            allowed_distance
        }
        None => max_distance,
    };

    // We wanted to move up to `max_toi` but can only move
    // `distance`.
    let factor = distance / (max_distance);

    transform.translation += direction * factor;
}

use game_wasm::components::builtin::{Collider, RigidBody};
use game_wasm::world::Entity;
use game_wasm::DT;

use crate::player::TransformChanged;

const G: f32 = -9.81;

pub fn drive_character_controller(entity: EntityId) {
    let entity = Entity::new(entity);

    let mut transform = entity.get::<Transform>().unwrap();
    let mut rigid_body = entity.get::<RigidBody>().unwrap();
    let collider = entity.get::<Collider>().unwrap();

    let prev_transform = transform;

    apply_gravity(
        entity.id(),
        &mut transform,
        &mut rigid_body.linvel,
        &collider.shape,
    );

    entity.insert(transform);
    entity.insert(rigid_body);

    if transform != prev_transform {
        dispatch_event(&TransformChanged {
            entity: entity.id(),
        });
    }
}

fn apply_gravity(
    entity: EntityId,
    transform: &mut Transform,
    linvel: &mut Vec3,
    shape: &ColliderShape,
) {
    let v0 = linvel.y;
    let altitude = transform.translation.y;

    let new_altitude = v0 * DT + altitude + 0.5 * G * DT * DT;

    //let max_toi = v0 * DT + 0.5 * G * DT * DT;
    //let max_distance = max_toi.abs();

    let delta = new_altitude - altitude;
    let direction = Vec3::Y * delta.signum();
    let max_toi = delta.abs();

    let filter = QueryFilter {
        exclude_entities: &[entity],
    };

    let (distance, hit) = match cast_shape(
        transform.translation,
        transform.rotation,
        direction,
        shape,
        // Note that `cast_shape` uses `gravity * max_toi` as the final max toi,
        // but `gravity` is already negative and `max_toi` must not be negative
        // for the shape cast to return useful results.
        // This also means that the returned `toi` and hit must be negated.
        max_toi + OFFSET,
        filter,
    ) {
        Some(hit) => {
            debug_assert!(hit.toi.is_sign_positive());
            let allowed_distance = f32::max(hit.toi - OFFSET, 0.0);

            (allowed_distance, true)
        }
        None => (max_toi, false),
    };

    transform.translation.y += distance * delta.signum();
    if hit {
        linvel.y = 0.0;
    } else {
        linvel.y += G * DT;
    }
}
