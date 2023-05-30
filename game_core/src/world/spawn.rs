use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Entity;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query};
use game_common::components::actor::{ActorFlag, ActorFlags, Spawn, SpawnPoints};
use game_common::components::combat::Health;
use game_common::components::transform::Transform;

/// The plugin responsible for spawning and respawning actors.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpawnPlugin;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(handle_spawns);
    }
}

fn handle_spawns(
    mut commands: Commands,
    mut actors: Query<
        (
            Entity,
            &mut Health,
            &mut ActorFlags,
            &mut Transform,
            &SpawnPoints,
        ),
        With<Spawn>,
    >,
) {
    for (entity, mut health, mut flags, mut transform, points) in &mut actors {
        // Skip this actor if it has no spawn point.
        let Some(point) = points.best() else {
            continue;
        };

        *health = Health::new(50);

        // If the actor respawned, it is no longer dead.
        flags.remove(ActorFlag::DEAD);

        flags.insert(ActorFlag::CAN_MOVE);
        flags.insert(ActorFlag::CAN_ROTATE);
        flags.insert(ActorFlag::CAN_ATTACK);

        transform.translation = point.translation;

        commands.entity(entity).remove::<Spawn>();
    }
}
