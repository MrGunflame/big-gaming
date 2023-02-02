use std::f32::consts::PI;
use std::mem::MaybeUninit;

use bevy::prelude::{
    Camera3d, Commands, CoreStage, Entity, EulerRot, EventReader, KeyCode, Plugin, Quat, Query,
    Res, ResMut, Transform, Vec3, With, Without,
};
use common::components::actor::{ActorState, MovementSpeed};
use common::components::movement::{Jump, Movement};
use common::components::player::HostPlayer;
use common::math::RotationExt;
use input::mouse::MouseMotion;

use crate::components::Rotation;
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
    mut players: Query<&mut MovementSpeed, With<HostPlayer>>,
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
    hotkeys: Res<HotkeyStore>,
    mut players: Query<(Entity, &Focus), With<HostPlayer>>,
) {
    let events = unsafe { EVENTS.assume_init_ref() };

    let (entity, focus) = players.single_mut();

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

    let mut angle = Angle::default();

    if hotkeys.triggered(events.forward) {
        angle.front();
    }

    if hotkeys.triggered(events.backward) {
        angle.back();
    }

    if hotkeys.triggered(events.right) {
        angle.right();
    }

    if hotkeys.triggered(events.left) {
        angle.left();
    }

    if let Some(angle) = angle.to_radians() {
        commands.entity(entity).insert(Movement {
            direction: Quat::from_axis_angle(Vec3::Y, angle),
        });
    }

    if hotkeys.triggered(events.jump) {
        // velocity.linvel.y += 1.0;
        commands.entity(entity).insert(Jump);
    }
}

/// The movement angle based on four input directions.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
struct Angle(u8);

impl Angle {
    const FRONT: Self = Self(1);
    const BACK: Self = Self(1 << 1);
    const LEFT: Self = Self(1 << 2);
    const RIGHT: Self = Self(1 << 3);

    fn front(&mut self) {
        self.0 |= Self::FRONT.0;
    }

    fn back(&mut self) {
        self.0 |= Self::BACK.0;
    }

    fn left(&mut self) {
        self.0 |= Self::LEFT.0;
    }

    fn right(&mut self) {
        self.0 |= Self::RIGHT.0;
    }

    fn to_degrees(self) -> Option<f32> {
        let (front, back, left, right) = (
            self.0 & Self::FRONT.0 != 0,
            self.0 & Self::BACK.0 != 0,
            self.0 & Self::LEFT.0 != 0,
            self.0 & Self::RIGHT.0 != 0,
        );

        match (front, back, left, right) {
            // Single
            (true, false, false, false) => Some(0.0),
            (false, true, false, false) => Some(180.0),
            (false, false, true, false) => Some(90.0),
            (false, false, false, true) => Some(270.0),
            // Front
            (true, false, true, false) => Some(45.0),
            (true, false, false, true) => Some(315.0),
            (true, false, true, true) => Some(0.0),
            // Back
            (false, true, true, false) => Some(135.0),
            (false, true, false, true) => Some(225.0),
            (false, true, true, true) => Some(180.0),
            // Locked
            (true, true, false, false) => None,
            (true, true, true, false) => Some(90.0),
            (true, true, false, true) => Some(270.0),
            (true, true, true, true) => None,

            (false, false, true, true) => None,
            (false, false, false, false) => None,
        }
    }

    fn to_radians(self) -> Option<f32> {
        self.to_degrees().map(f32::to_radians)
    }
}

fn mouse_movement(
    mut events: EventReader<MouseMotion>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
    mut players: Query<(&mut Transform, &Focus), (With<HostPlayer>, Without<Camera3d>)>,
) {
    let mut camera = cameras.single_mut();
    let (mut player, _) = players.single_mut();

    for event in events.iter() {
        let yaw = event.delta.x * 0.001;
        let pitch = event.delta.y * 0.001;

        let yaw = camera.rotation.yaw() + yaw;
        let mut pitch = camera.rotation.pitch() + pitch;

        if pitch < -(PI / 2.0) {
            pitch = -(PI / 2.0);
        } else if pitch > PI / 2.0 {
            pitch = PI / 2.0;
        }

        dbg!(camera.rotation.to_euler(EulerRot::YXZ));

        // let quat = camera.rotation.with_yaw(yaw).with_pitch(pitch);
        let quat = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
        dbg!(quat.to_euler(EulerRot::YXZ));

        // let quat = Quat::from_euler(EulerRot::YXZ, y - yaw, pitch, z);

        // camera.rotation.to_axis_angle();
        // camera.rotate_axis(-Vec3::Y, yaw);
        // camera.rotate(quat);
        camera.rotation = quat;

        // *camera_rot = camera_rot
        //     .add_yaw(Degrees(yaw))
        //     .saturating_add_pitch(Degrees(pitch));

        // *player_rot = camera_rot.with_pitch(Radians(0.0));
        player.rotation = camera.rotation;
    }
}
