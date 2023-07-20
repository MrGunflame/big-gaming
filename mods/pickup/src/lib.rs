use game_wasm::entity::EntityId;
use game_wasm::events::on_action;
use game_wasm::math::{Ray, Vec3};
use game_wasm::physics::cast_ray;
use game_wasm::world::Entity;

#[on_action]
fn on_action(entity: u64, invoker: u64) {
    let id = EntityId::from_raw(invoker);
    let entity = Entity::get(id).unwrap();

    let translation = entity.translation();
    let direction = entity.rotation() * -Vec3::Z;
    let ray = Ray {
        origin: translation,
        direction,
    };

    let target = match cast_ray(ray, 3.0) {
        Some(hit) => hit.entity,
        None => return,
    };

    Entity::get(target).unwrap().despawn().unwrap();
}
