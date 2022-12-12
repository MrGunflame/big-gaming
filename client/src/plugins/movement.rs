use std::mem::MaybeUninit;

use bevy::prelude::{
    Component, Entity, KeyCode, Plugin, Query, Res, ResMut, Transform, Vec3, With,
};
use bevy::time::Time;
use bevy_rapier3d::prelude::{Collider, QueryFilter, RapierContext, Velocity};

use crate::components::{ActorState, Rotation};
use crate::entities::player::PlayerCharacter;
use crate::utils::Degrees;

use super::hotkeys::{Event, EventId, HotkeyStore};

const DEFAULT_TRIGGER_FORWARD: KeyCode = KeyCode::W;
const DEFAULT_TRIGGER_BACKWARD: KeyCode = KeyCode::S;
const DEFAULT_TRIGGER_LEFT: KeyCode = KeyCode::A;
const DEFAULT_TRIGGER_RIGHT: KeyCode = KeyCode::D;
const DEFAULT_TRIGGER_JUMP: KeyCode = KeyCode::Space;

static mut EVENTS: MaybeUninit<Events> = MaybeUninit::uninit();

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(register_events)
            .add_system(movement_events);
    }
}

#[derive(Copy, Clone, Debug)]
struct Events {
    forward: EventId,
    backward: EventId,
    left: EventId,
    right: EventId,
    jump: EventId,
}

fn register_events(mut hotkeys: ResMut<HotkeyStore>) {
    let forward = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_FORWARD));
    let backward = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_BACKWARD));
    let left = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_LEFT));
    let right = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_RIGHT));
    let jump = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_JUMP));

    unsafe {
        EVENTS.write(Events {
            forward,
            backward,
            left,
            right,
            jump,
        });
    }
}

fn movement_events(
    time: Res<Time>,
    rapier: Res<RapierContext>,
    hotkeys: Res<HotkeyStore>,
    mut players: Query<
        (
            Entity,
            &mut Transform,
            &Rotation,
            &mut Velocity,
            &MovementSpeed,
            &Collider,
            &ActorState,
        ),
        With<PlayerCharacter>,
    >,
) {
    let delta = time.delta_seconds();

    let events = unsafe { EVENTS.assume_init_ref() };

    let (entity, mut transform, rotation, mut velocity, speed, collider, state) =
        players.single_mut();

    // Only process movement events while the actor in the default state.
    if *state != ActorState::NORMAL {
        return;
    }

    let mut vec = Vec3::ZERO;

    let shape_pos = transform.translation;
    let shape_rot = transform.rotation;
    let is_on_ground = || {
        let shape_vel = -Vec3::Y;
        let max_toi = 2.0;
        let filter = QueryFilter::new().exclude_collider(entity);

        rapier
            .cast_shape(shape_pos, shape_rot, shape_vel, &collider, max_toi, filter)
            .is_some()
    };

    if hotkeys.triggered(events.forward) {
        vec += rotation.movement_vec();
    }

    if hotkeys.triggered(events.backward) {
        vec += rotation.left(Degrees(180.0)).movement_vec();
    }

    if hotkeys.triggered(events.left) {
        vec += rotation.left(Degrees(90.0)).movement_vec();
    }

    if hotkeys.triggered(events.right) {
        vec += rotation.right(Degrees(90.0)).movement_vec();
    }

    transform.translation += vec * delta * speed.0;

    if hotkeys.triggered(events.jump) {
        if is_on_ground() {
            velocity.linvel.y += 10.0;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Component)]
#[repr(transparent)]
pub struct MovementSpeed(pub f32);
