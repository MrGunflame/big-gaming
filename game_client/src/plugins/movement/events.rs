use bevy::prelude::{Commands, Entity, Query, Res, Transform};
use bevy::time::Time;
use game_common::components::actor::{ActorFlag, ActorFlags, MovementSpeed};
use game_common::components::movement::{Movement, Rotate};
use game_common::math::RotationExt;
use game_net::snapshot::Command;

use crate::net::ServerConnection;

pub fn handle_movement_events(
    conn: Res<ServerConnection>,
    mut commands: Commands,
    time: Res<Time>,
    mut actors: Query<(
        Entity,
        &ActorFlags,
        &mut Transform,
        &MovementSpeed,
        &Movement,
    )>,
) {
    let delta = time.delta_seconds();

    for (entity, flags, mut transform, speed, movement) in &mut actors {
        if !flags.contains(ActorFlag::CAN_MOVE) {
            continue;
        }

        let rotation = transform.rotation * movement.direction;
        transform.translation += rotation.dir_vec() * speed.0 * delta;

        // Inform the server that we want to move the entity.
        if let Some(id) = conn.lookup(entity) {
            conn.send(Command::EntityTranslate {
                id,
                translation: transform.translation,
            });
        }

        commands.entity(entity).remove::<Movement>();
    }
}

pub fn handle_rotate_events(
    mut commands: Commands,
    conn: Res<ServerConnection>,
    mut actors: Query<(Entity, &ActorFlags, &mut Transform, &Rotate)>,
) {
    for (entity, flags, mut transform, rotate) in &mut actors {
        if !flags.contains(ActorFlag::CAN_ROTATE) {
            continue;
        }

        transform.rotation = rotate.destination;

        if let Some(id) = conn.lookup(entity) {
            conn.send(Command::EntityRotate {
                id,
                rotation: transform.rotation,
            });
        }

        commands.entity(entity).remove::<Rotate>();
    }
}
