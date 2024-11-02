use game_wasm::components::builtin::{Collider, Transform};
use game_wasm::entity::EntityId;
use game_wasm::math::Vec3;
use game_wasm::physics::{cast_shape, QueryFilter, RayHit};
use game_wasm::world::Entity;

use crate::collect_children_recursive;

pub fn cast_actor(entity: EntityId, direction: Vec3, max_toi: f32) -> Option<RayHit> {
    let entity = Entity::new(entity);

    let Ok(transform) = entity.get::<Transform>() else {
        return None;
    };
    let Ok(collider) = entity.get::<Collider>() else {
        return None;
    };

    let mut exclude_entities = collect_children_recursive(entity.id());
    exclude_entities.push(entity.id());

    cast_shape(
        transform.translation,
        transform.rotation,
        direction,
        &collider.shape,
        max_toi,
        QueryFilter {
            exclude_entities: &exclude_entities,
        },
    )
}
