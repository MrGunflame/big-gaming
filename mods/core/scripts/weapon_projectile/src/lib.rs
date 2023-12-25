use game_wasm::entity::EntityId;
use game_wasm::events::on_collision;
use game_wasm::world::Entity;
use shared::{Health, ProjectileProperties};

#[on_collision]
fn on_collision(entity: EntityId, other: EntityId) {
    let entity = Entity::new(entity);

    let props = entity.get::<ProjectileProperties>().unwrap();
    entity.despawn();

    let target = Entity::new(other);

    let mut health = target.get::<Health>().unwrap();
    health.0 -= props.damage;
    target.insert(health);
}
