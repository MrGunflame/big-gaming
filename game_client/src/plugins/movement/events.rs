use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Transform, With};
use bevy::time::Time;
use bevy_rapier3d::prelude::Velocity;
use game_common::components::actor::{ActorFlag, ActorFlags, ActorProperties, MovementSpeed};
use game_common::components::movement::{Jump, Movement, Rotate, RotateQueue};
use game_common::entity::EntityMap;
use game_common::math::RotationExt;
use game_net::snapshot::Command;
use game_net::world::WorldState;

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
    mut world: ResMut<WorldState>,
    map: ResMut<EntityMap>,
) {
    let delta = time.delta_seconds();

    for (entity, flags, mut transform, speed, movement) in &mut actors {
        if !flags.contains(ActorFlag::CAN_MOVE) {
            continue;
        }

        let rotation = transform.rotation * movement.direction;
        let translation = rotation.dir_vec() * speed.0 * delta;

        transform.translation += translation;

        // Inform the server that we want to move the entity.
        if let Some(id) = conn.lookup(entity) {
            conn.send(Command::EntityTranslate {
                id,
                translation: transform.translation,
            });
        }

        commands.entity(entity).remove::<Movement>();

        let period = conn.interpolation_period();

        let mut view = world.get_mut(period).unwrap();
        let mut ent = view.get_mut(map.get_entity(entity).unwrap()).unwrap();
        ent.transform.translation = transform.translation;

        drop(ent);
        drop(view);
        world.patch_delta(period);
    }
}

pub fn handle_rotate_events(
    mut commands: Commands,
    conn: Res<ServerConnection>,
    mut actors: Query<(Entity, &ActorFlags, &mut ActorProperties, &mut RotateQueue)>,
    mut world: ResMut<WorldState>,
    map: ResMut<EntityMap>,
) {
    for (entity, flags, mut props, mut rotate) in &mut actors {
        if !flags.contains(ActorFlag::CAN_ROTATE) {
            continue;
        }

        let mut changed = false;

        while let Some(dest) = rotate.0.pop_front() {
            changed = true;
            // props.rotation *= dest.destination;
        }

        if changed {
            if let Some(id) = conn.lookup(entity) {
                conn.send(Command::EntityRotate {
                    id,
                    rotation: props.rotation,
                });
            }

            let period = conn.interpolation_period();

            let mut view = world.get_mut(period).unwrap();
            let mut ent = view.get_mut(map.get_entity(entity).unwrap()).unwrap();
            ent.transform.rotation = props.rotation;

            drop(ent);
            drop(view);
            world.patch_delta(period);
        }
    }
}

pub fn handle_jump_events(
    mut commands: Commands,
    conn: Res<ServerConnection>,
    mut actors: Query<(Entity, &ActorFlags, &mut Velocity), With<Jump>>,
) {
    for (entity, flags, mut velocity) in &mut actors {
        if !flags.contains(ActorFlag::CAN_MOVE) {
            continue;
        }

        velocity.linvel.y += 10.0;

        if let Some(id) = conn.lookup(entity) {
            conn.send(Command::EntityVelocity {
                id,
                linvel: velocity.linvel,
                angvel: velocity.angvel,
            });
        }

        commands.entity(entity).remove::<Jump>();
    }
}
