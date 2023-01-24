use bevy::prelude::{Commands, Entity, Plugin, Query, Transform, With};
use common::components::actor::{ActorFlag, ActorFlags, Spawn, SpawnPoints};

/// The plugin responsible for spawning and respawning actors.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpawnPlugin;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(handle_spawns);
    }
}

fn handle_spawns(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut ActorFlags, &mut Transform, &SpawnPoints), With<Spawn>>,
) {
    for (entity, mut flags, mut transform, points) in &mut actors {
        // Skip this actor if it has no spawn point.
        let Some(point) = points.best() else {
            continue;
        };

        // If the actor respawned, it is no longer dead.
        flags.remove(ActorFlag::DEAD);

        flags.insert(ActorFlag::CAN_MOVE);
        flags.insert(ActorFlag::CAN_ROTATE);
        flags.insert(ActorFlag::CAN_ATTACK);

        transform.translation = point.translation;

        commands.entity(entity).remove::<Spawn>();
    }
}
