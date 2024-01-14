#![no_std]

use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::math::{Ray, Vec3};
use game_wasm::physics::{cast_ray, QueryFilter};
use game_wasm::world::Entity;
use shared::panic_handler;

//panic_handler!();

// #[on_action]
// fn on_action(invoker: EntityId) {
//     let entity = Entity::get(invoker).unwrap();

//     let translation = entity.translation();
//     let direction = entity.rotation() * -Vec3::Z;
//     let ray = Ray {
//         origin: translation,
//         direction,
//     };

//     let filter = QueryFilter {
//         exclude_entities: &[invoker],
//     };

//     let target = match cast_ray(ray, 5.0, filter) {
//         Some(hit) => hit.entity,
//         None => return,
//     };

//     let entity = Entity::get(target).unwrap();
//     if !entity.kind().is_item() {
//         return;
//     }

//     entity.despawn().unwrap();
// }
