use std::collections::VecDeque;

use bevy::prelude::{App, Commands, Entity, Quat, Query, Res, Transform, Vec3};
use bevy::time::Time;
use bevy_ecs::component::Component;
use bevy_ecs::system::EntityCommands;
use common::components::actor::MovementSpeed;

pub(super) fn actions(app: &mut App) {
    app.add_system(turn)
        .add_system(attack)
        .add_system(drive_steps)
        .add_system(drive_concurrent);
}

pub trait Action: Send + Sync + 'static {
    fn build(&mut self, commands: &mut EntityCommands<'_, '_, '_>);
}

#[derive(Copy, Clone, Debug, Component)]
pub struct Translate {
    pub target: Vec3,
}

impl Action for Translate {
    fn build(&mut self, commands: &mut EntityCommands<'_, '_, '_>) {
        commands.insert(*self);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Component)]
pub struct Rotate {
    pub target: Quat,
}

impl Action for Rotate {
    fn build(&mut self, commands: &mut EntityCommands) {
        commands.insert(*self);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Component)]
pub struct Attack {
    pub location: Vec3,
}

impl Action for Attack {
    fn build(&mut self, commands: &mut EntityCommands) {
        commands.insert(*self);
    }
}

#[derive(Component)]
pub struct Steps {
    actions: VecDeque<Box<dyn Action>>,
}

impl Steps {
    pub fn push<T>(&mut self, action: T)
    where
        T: Action,
    {
        self.push_boxed(Box::new(action));
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn pop(&mut self, commands: &mut EntityCommands<'_, '_, '_>) {
        if let Some(mut action) = self.actions.pop_front() {
            action.build(commands);
        }
    }

    fn push_boxed(&mut self, action: Box<dyn Action>) {
        self.actions.push_back(action);
    }
}

#[derive(Component)]
pub struct Concurrent {
    actions: VecDeque<Box<dyn Action>>,
}

impl Concurrent {
    pub fn push<T>(&mut self, action: T)
    where
        T: Action,
    {
        self.push_boxed(Box::new(action));
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn pop(&mut self, commands: &mut EntityCommands<'_, '_, '_>) {
        if let Some(mut action) = self.actions.pop_front() {
            action.build(commands);
        }
    }

    fn push_boxed(&mut self, action: Box<dyn Action>) {
        self.actions.push_front(action);
    }
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

fn attack(
    mut time: Res<Time>,
    mut commands: Commands,
    mut hosts: Query<(Entity, &mut Transform, &Attack)>,
) {
    for (entity, mut transform, attack) in &mut hosts {}
}

fn drive_steps(mut commands: Commands, mut hosts: Query<(Entity, &mut Steps)>) {
    for (entity, mut steps) in &mut hosts {
        if !steps.is_empty() {
            steps.pop(&mut commands.entity(entity));
        }
    }
}

fn drive_concurrent(mut commands: Commands, mut hosts: Query<(Entity, &mut Concurrent)>) {
    for (entity, mut concurrent) in &mut hosts {
        while !concurrent.is_empty() {
            concurrent.pop(&mut commands.entity(entity));
        }
    }
}
