use bevy::prelude::{Commands, Entity, Plugin, Query, Transform};
use common::components::movement::{Movement, Rotate, Teleport};

// FIXME: Different behaivoir in client/server envs
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(handle_movement_events)
            .add_system(handle_rotate_events)
            .add_system(handle_teleport_events);
    }
}

fn handle_movement_events(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Transform, &Movement)>,
) {
    for (entity, mut transform, movement) in &mut actors {
        transform.translation = movement.desination;

        commands.entity(entity).remove::<Movement>();
    }
}

fn handle_rotate_events(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Transform, &Rotate)>,
) {
    for (entity, mut transform, rotate) in &mut actors {
        transform.rotation = rotate.destination;

        commands.entity(entity).remove::<Rotate>();
    }
}

fn handle_teleport_events(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Transform, &Teleport)>,
) {
    for (entity, mut transform, teleport) in &mut actors {
        transform.translation = teleport.destination;

        commands.entity(entity).remove::<Teleport>();
    }
}
