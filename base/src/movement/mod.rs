use std::time::Duration;

use bevy::prelude::{
    Commands, Component, Entity, EulerRot, Plugin, Query, Res, Transform, Vec3, With,
};
use bevy::time::Time;
use bevy_rapier3d::prelude::Velocity;
use common::components::actor::MovementSpeed;
use common::components::items::Cooldown;
use common::components::movement::{Jump, Movement, Rotate, Teleport};

// FIXME: Different behaivoir in client/server envs
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(handle_movement_events)
            .add_system(handle_rotate_events)
            .add_system(handle_teleport_events)
            .add_system(handle_jump_events);
    }
}

fn handle_movement_events(
    mut commands: Commands,
    time: Res<Time>,
    mut actors: Query<(Entity, &mut Transform, &MovementSpeed, &Movement)>,
) {
    let delta = time.delta_seconds();

    for (entity, mut transform, speed, movement) in &mut actors {
        let rotation = transform.rotation * movement.direction;
        let (y, x, _) = rotation.to_euler(EulerRot::YXZ);
        let dir = Vec3::new(-y.sin() * x.cos(), x.sin(), -y.cos() * x.cos()).normalize();

        transform.translation += dir * speed.0 * delta;

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

fn handle_jump_events(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Velocity, Option<&mut JumpCooldown>), With<Jump>>,
) {
    for (entity, mut velocity, cooldown) in &mut actors {
        // FIXME: Maybe the actor should always have the JumpCooldown already
        // to avoid this extra check.
        if let Some(mut cooldown) = cooldown {
            if !cooldown.cooldown.tick() {
                return;
            }
        } else {
            commands.entity(entity).insert(JumpCooldown::new());
        }

        velocity.linvel.y += 10.0;

        commands.entity(entity).remove::<Jump>();
    }
}

#[derive(Copy, Clone, Debug, Component)]
struct JumpCooldown {
    cooldown: Cooldown,
}

impl JumpCooldown {
    fn new() -> Self {
        Self {
            cooldown: Cooldown::new(Duration::from_secs(1)),
        }
    }
}
