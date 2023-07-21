use game_common::components::combat::Health;
use game_common::components::components::Components;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::world::entity::{Actor, Entity, EntityBody};
use game_common::world::world::WorldViewMut;
use glam::Vec3;

pub fn spawn_player(view: &mut WorldViewMut<'_>) -> SpawnPlayer {
    let race = "0a73147476444aba90f71207b22d7419:02".parse().unwrap();

    let transform = Transform::from_translation(Vec3::new(10.0, 32.0, 10.0));

    let id = view.spawn(Entity {
        id: EntityId::dangling(),
        transform,
        body: EntityBody::Actor(Actor {
            race: RaceId(race),
            health: Health::new(50),
        }),
        components: Components::new(),
    });

    SpawnPlayer { id, transform }
}

#[derive(Copy, Clone, Debug)]
pub struct SpawnPlayer {
    pub id: EntityId,
    pub transform: Transform,
}
