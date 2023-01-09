use bevy::prelude::{App, Commands, Entity, Quat, Query, Res, Transform, Vec3};
use bevy::time::Time;
use bevy_ecs::component::Component;
use common::components::actor::MovementSpeed;

pub(super) fn actions(app: &mut App) {
    app.add_system(turn);
}

pub struct Action {}

#[derive(Copy, Clone, Debug, Component)]
pub struct Translate {
    pub target: Vec3,
}

#[derive(Copy, Clone, Debug, PartialEq, Component)]
pub struct Rotate {
    pub target: Quat,
}

fn turn(
    mut time: Res<Time>,
    mut commands: Commands,
    mut hosts: Query<(Entity, &mut Transform, &MovementSpeed, &Rotate)>,
) {
    for (entity, mut transform, speed, rotate) in &mut hosts {
        let delta = *speed * time.delta();

        transform.rotation = rotate.target;
        commands.entity(entity).remove::<Rotate>();
    }
}
