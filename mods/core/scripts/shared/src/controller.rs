use game_wasm::components::builtin::{ColliderShape, Transform};
use game_wasm::entity::EntityId;
use game_wasm::math::Vec3;
use game_wasm::physics::{cast_shape, QueryFilter};

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

    // Small offset that prevents the character getting stuck on the
    // ground if the collider "glitches" into the ground.
    let offset = Vec3::new(0.0, 0.1, 0.0);

    let distance = match cast_shape(
        transform.translation + offset,
        transform.rotation,
        direction,
        shape,
        max_toi,
        filter,
    ) {
        Some(hit) => hit.toi,
        None => max_toi,
    };

    // We wanted to move up to `max_toi` but can only move
    // `distance`.
    let factor = distance / max_toi;

    transform.translation += direction * factor;
}
