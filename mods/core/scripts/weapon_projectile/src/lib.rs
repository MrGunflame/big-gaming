use game_wasm::components::builtin::Transform;
use game_wasm::entity::EntityId;
use game_wasm::events::on_update;
use game_wasm::math::Ray;
use game_wasm::physics::{cast_ray, QueryFilter};
use game_wasm::world::Entity;
use shared::{Health, ProjectileProperties, Vec3};

// Note that we manually drive the projectile and don't delegate that task
// to the physics engine to prevent weird side-effects. Adding a collider to
// the projectile would imply that it could get be deflected by other entities
// while also being a "solid" object that would block movement.
#[on_update]
fn on_update(entity: EntityId) {
    let entity = Entity::new(entity);

    let mut transform = entity.get::<Transform>();
    let props = entity.get::<ProjectileProperties>();

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
        Some(hit) => {
            entity.despawn();

            // let target = Entity::new(hit.entity);
            // let mut health = target.get::<Health>();
            // health.0 -= props.damage;
            // target.insert(health);
        }
        None => {
            transform.translation += transform.rotation * -Vec3::Z;
            entity.insert(transform);
        }
    }
}
