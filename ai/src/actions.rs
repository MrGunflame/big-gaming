use bevy::prelude::{App, Quat, Query, Res, Transform, Vec3};
use bevy::time::Time;
use bevy_ecs::component::Component;
use common::components::actor::MovementSpeed;

pub(super) fn actions(app: &mut App) {
    app.add_system(turn);
}

pub struct Action {}

#[derive(Component)]
pub struct Move {
    translation: Vec3,
}

#[derive(Component)]
pub struct Rotate {
    pub rotation: Quat,
}

fn turn(mut time: Res<Time>, mut hosts: Query<(&mut Transform, &MovementSpeed, &Rotate)>) {
    for (mut transform, speed, turn) in &mut hosts {
        let delta = *speed * time.delta();

        transform.rotation = turn.rotation;
    }
}
