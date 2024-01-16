use game_wasm::components::builtin::Transform;
use game_wasm::entity::EntityId;
use game_wasm::math::{Ray, Vec3};
use game_wasm::physics::{cast_ray, QueryFilter};
use game_wasm::world::Entity;

use crate::{apply_actor_damage, ProjectileProperties};

// Note that we manually drive the projectile and don't delegate that task
// to the physics engine to prevent weird side-effects. Adding a collider to
// the projectile would imply that it could get be deflected by other entities
// while also being a "solid" object that would block movement.
pub fn drive_projectile(entity: EntityId) {
    let entity = Entity::new(entity);

    let mut transform = entity.get::<Transform>().unwrap();
    let props = entity.get::<ProjectileProperties>().unwrap();

    let max_toi = 10.0;
    let filter = QueryFilter {
        exclude_entities: &[],
    };

    match cast_ray(
        Ray {
            origin: transform.translation,
            direction: transform.rotation * -Vec3::Z,
        },
        max_toi,
        filter,
    ) {
        // Ignore hitting the owner of the projectile which will usually
        // always happen at TOI=0 when the projectile is spawned at the
        // actor.
        Some(hit) if hit.entity != props.owner => {
            entity.despawn();

            let target = Entity::new(hit.entity);
            apply_actor_damage(props.damage as u32, target);
        }
        Some(_) | None => {
            transform.translation += transform.rotation * -Vec3::Z;
            entity.insert(transform);
        }
    }
}
