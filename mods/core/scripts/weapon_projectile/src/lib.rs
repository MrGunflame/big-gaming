use game_wasm::entity::EntityId;
use game_wasm::events::on_collision;
use game_wasm::world::Entity;
use shared::components::{HEALTH, PROJECTILE_PROPERTIES};
use shared::{Health, ProjectileProperties};

// #[on_collision]
// fn on_collision(entity: EntityId, other: EntityId) {
//     let entity = Entity::get(entity).unwrap();

//     let props = entity
//         .components()
//         .get(PROJECTILE_PROPERTIES)
//         .unwrap()
//         .read::<ProjectileProperties>();
//     entity.despawn().unwrap();

//     let target = Entity::get(other).unwrap();

//     let Ok(mut health) = target.components().get(HEALTH) else {
//         return;
//     };

//     health.update::<Health, _, _>(|health| health.0 -= props.damage);

//     target.components().insert(HEALTH, &health).unwrap();
// }
