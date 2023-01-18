use std::mem::MaybeUninit;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::{
    Camera3d, Commands, Component, CoreStage, Entity, EventReader, KeyCode, Plugin, Query, Res,
    ResMut, Transform, Vec3, With, Without,
};
use bevy::time::Time;
use bevy_rapier3d::prelude::{Collider, QueryFilter, RapierContext, Velocity};
use common::components::actor::{ActorState, MovementSpeed};
use common::components::movement::Jump;

use crate::components::Rotation;
use crate::entities::player::PlayerCharacter;
use crate::ui::{Focus, FocusKind};
use crate::utils::{Degrees, Radians};

use super::hotkeys::{Event, EventId, HotkeyStore};

const DEFAULT_TRIGGER_FORWARD: KeyCode = KeyCode::W;
const DEFAULT_TRIGGER_BACKWARD: KeyCode = KeyCode::S;
const DEFAULT_TRIGGER_LEFT: KeyCode = KeyCode::A;
const DEFAULT_TRIGGER_RIGHT: KeyCode = KeyCode::D;
const DEFAULT_TRIGGER_JUMP: KeyCode = KeyCode::Space;
const DEFAULT_TRIGGER_SPRINT: KeyCode = KeyCode::LShift;

static mut EVENTS: MaybeUninit<Events> = MaybeUninit::uninit();

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(register_events)
            // Run in PreUpdate before camera is updated.
            .add_system_to_stage(CoreStage::PreUpdate, movement_events)
            .add_system(mouse_movement)
            .add_system(toggle_sprint);
    }
}

#[derive(Copy, Clone, Debug)]
struct Events {
    forward: EventId,
    backward: EventId,
    left: EventId,
    right: EventId,
    jump: EventId,
    sprint: EventId,
}

fn register_events(mut hotkeys: ResMut<HotkeyStore>) {
    let forward = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_FORWARD));
    let backward = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_BACKWARD));
    let left = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_LEFT));
    let right = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_RIGHT));
    let jump = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_JUMP));
    let sprint = hotkeys.register(Event::new().trigger(DEFAULT_TRIGGER_SPRINT));

    unsafe {
        EVENTS.write(Events {
            forward,
            backward,
            left,
            right,
            jump,
            sprint,
        });
    }
}

fn toggle_sprint(
    hotkeys: Res<HotkeyStore>,
    mut players: Query<&mut MovementSpeed, With<PlayerCharacter>>,
) {
    let events = unsafe { EVENTS.assume_init_ref() };

    let mut speed = players.single_mut();

    if hotkeys.triggered(events.sprint) {
        **speed = 5.0;
    } else {
        **speed = 3.0;
    }
}

fn movement_events(
    mut commands: Commands,
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
            // &Collider,
            &ActorState,
            &Focus,
        ),
        With<PlayerCharacter>,
    >,
) {
    let delta = time.delta_seconds();

    let events = unsafe { EVENTS.assume_init_ref() };

    let (entity, mut transform, rotation, mut velocity, speed, state, focus) = players.single_mut();

    // Only process movement events while the actor in the default state.
    if *state != ActorState::DEFAULT || focus.kind != FocusKind::World {
        return;
    }

    let mut vec = Vec3::ZERO;

    // let shape_pos = transform.translation;
    // let shape_rot = transform.rotation;
    // let is_on_ground = || {
    //     let shape_vel = -Vec3::Y;
    //     let max_toi = 2.0;
    //     let filter = QueryFilter::new().exclude_collider(entity);

    //     rapier
    //         .cast_shape(shape_pos, shape_rot, shape_vel, &collider, max_toi, filter)
    //         .is_some()
    // };

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
        // velocity.linvel.y += 1.0;
        commands.entity(entity).insert(Jump);
    }
}

fn mouse_movement(
    mut events: EventReader<MouseMotion>,
    mut cameras: Query<&mut Rotation, With<Camera3d>>,
    mut players: Query<
        (&mut Rotation, &ActorState, &Focus),
        (With<PlayerCharacter>, Without<Camera3d>),
    >,
) {
    let mut camera_rot = cameras.single_mut();
    let (mut player_rot, state, focus) = players.single_mut();

    if *state != ActorState::DEFAULT || focus.kind != FocusKind::World {
        return;
    }

    for event in events.iter() {
        let yaw = event.delta.x * 0.1;
        let pitch = event.delta.y * 0.1;

        *camera_rot = camera_rot
            .add_yaw(Degrees(yaw))
            .saturating_add_pitch(Degrees(pitch));

        *player_rot = camera_rot.with_pitch(Radians(0.0));
    }
}
