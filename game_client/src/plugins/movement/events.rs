use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{Commands, Query, Res, ResMut};
use game_common::components::actor::{ActorFlag, ActorFlags, ActorProperties, MovementSpeed};
use game_common::components::movement::{Movement, RotateQueue};
use game_common::components::transform::Transform;
use game_common::math::RotationExt;
use game_common::world::world::WorldState;
use game_core::time::Time;
use game_net::snapshot::Command;

use crate::net::ServerConnection;

pub fn handle_movement_events(
    mut conn: ResMut<ServerConnection>,
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
) {
    let delta = time.delta().as_secs_f32();

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

        let Some(mut view) = world.back_mut() else {
            return;
        };

        let id = conn.entities.get_entity(entity).unwrap();

        // The host entity may not exist yet. (If the player was spawned before the rendering
        // interpolation period was reached.)
        let Some(mut ent) = view.get_mut(id) else {
            // The entity should already exists in the newest view.
            #[cfg(debug_assertions)]
            {
                drop(view);
                assert!(world.front().unwrap().get(id).is_some());
            }

            return;
        };

        ent.transform.translation = transform.translation;

        drop(ent);
        drop(view);
    }
}

pub fn handle_rotate_events(
    mut conn: ResMut<ServerConnection>,
    mut actors: Query<(Entity, &ActorFlags, &mut ActorProperties, &mut RotateQueue)>,
    mut world: ResMut<WorldState>,
) {
    for (entity, flags, props, mut rotate) in &mut actors {
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

            let Some(mut view) = world.back_mut() else {
                return;
            };

            let id = conn.entities.get_entity(entity).unwrap();

            // The host entity may not exist yet. (If the player was spawned before the rendering
            // interpolation period was reached.)
            let Some(mut ent) = view.get_mut(id) else {
                // The entity should already exists in the newest view.
                #[cfg(debug_assertions)]
                {
                    drop(view);
                    assert!(world.front().unwrap().get(id).is_some());
                }

                return;
            };

            ent.transform.rotation = props.rotation;

            drop(ent);
            drop(view);
        }
    }
}

// pub fn handle_jump_events(
//     mut commands: Commands,
//     conn: Res<ServerConnection>,
//     mut actors: Query<(Entity, &ActorFlags), With<Jump>>,
// ) {
//     for (entity, flags, mut velocity) in &mut actors {
//         if !flags.contains(ActorFlag::CAN_MOVE) {
//             continue;
//         }

//         velocity.linvel.y += 10.0;

//         if let Some(id) = conn.lookup(entity) {
//             conn.send(Command::EntityVelocity {
//                 id,
//                 linvel: velocity.linvel,
//                 angvel: velocity.angvel,
//             });
//         }

//         commands.entity(entity).remove::<Jump>();
//     }
// }
