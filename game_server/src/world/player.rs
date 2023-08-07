use game_common::components::combat::Health;
use game_common::components::components::Components;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::world::entity::{Actor, Entity, EntityBody};
use game_common::world::world::WorldViewMut;
use game_net::snapshot::PlayerMove;
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
        is_host: false,
    });

    SpawnPlayer { id, transform }
}

#[derive(Copy, Clone, Debug)]
pub struct SpawnPlayer {
    pub id: EntityId,
    pub transform: Transform,
}

pub fn move_player(event: PlayerMove, entity_id: EntityId, view: &mut WorldViewMut<'_>) {
    let Some(mut entity) = view.get_mut(entity_id) else {
        return;
    };

    let speed = 1.0;

    // FIXME: This is not quite correct, if the entity moves along two axes it
    // should not move along both with the speed as if it were moving into one
    // direction. (i.e. Forward+Left moves the player less along both the Forward
    // and left axes than just a Foward/Left command).
    let dir = (event.bits.forward as u8 as f32) * -Vec3::Z
        + (event.bits.back as u8 as f32) * Vec3::Z
        + (event.bits.left as u8 as f32) * -Vec3::X
        + (event.bits.right as u8 as f32) * Vec3::X;

    let delta = entity.transform.rotation * dir * speed;
    entity.set_translation(entity.transform.translation + delta);
}
