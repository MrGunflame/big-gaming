use std::mem::MaybeUninit;

use bevy::input::mouse::MouseWheel;
use bevy::prelude::{EventReader, KeyCode, Query, Res, ResMut};
use bevy::time::Time;

use crate::entities::player::CameraPosition;
use crate::plugins::hotkeys::{Event, EventId, HotkeyStore, TriggerKind};

const DEFAULT_TRIGGER_TOGGLE_CAMERA: KeyCode = KeyCode::V;

static mut EVENTS: MaybeUninit<Events> = MaybeUninit::uninit();

struct Events {
    toggle_camera: EventId,
}

pub(super) fn register_events(mut hotkeys: ResMut<HotkeyStore>) {
    let toggle_camera = hotkeys.register(
        Event::new()
            .trigger(DEFAULT_TRIGGER_TOGGLE_CAMERA)
            .kind(TriggerKind::Trigger),
    );

    unsafe {
        EVENTS.write(Events { toggle_camera });
    }
}

pub(super) fn toggle_camera_position(
    hotkeys: Res<HotkeyStore>,
    mut cameras: Query<&mut CameraPosition>,
) {
    let events = unsafe { EVENTS.assume_init_ref() };

    let mut position = cameras.single_mut();

    if hotkeys.triggered(events.toggle_camera) {
        *position = match *position {
            CameraPosition::FirstPerson => CameraPosition::ThirdPerson { distance: 5.0 },
            CameraPosition::ThirdPerson { distance: _ } => CameraPosition::FirstPerson,
        };
    }
}

pub(super) fn adjust_camera_distance(
    time: Res<Time>,
    mut cameras: Query<&mut CameraPosition>,
    mut events: EventReader<MouseWheel>,
) {
    let mut position = cameras.single_mut();

    let delta = time.delta_seconds();

    if let CameraPosition::ThirdPerson { distance } = &mut *position {
        for event in events.iter() {
            *distance -= event.y * delta * 5.0;

            if *distance < 1.0 {
                *distance = 1.0;
            } else if *distance > 10.0 {
                *distance = 10.0;
            }
        }
    }
}
