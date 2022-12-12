use std::mem::MaybeUninit;

use bevy::prelude::{KeyCode, Query, Res, ResMut};

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
