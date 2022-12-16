use std::mem::MaybeUninit;

use bevy::prelude::{KeyCode, Res, ResMut};

use crate::plugins::hotkeys::{Event, EventId, HotkeyStore, TriggerKind};

use super::interfaces::{MENU_DEATH, MENU_GAME};
use super::InterfaceState;

const DEFAULT_TRIGGER_GAMEMENU: KeyCode = KeyCode::Escape;

static mut EVENTS: MaybeUninit<Events> = MaybeUninit::uninit();

struct Events {
    game_menu: EventId,
}

pub(super) fn register_events(mut hotkeys: ResMut<HotkeyStore>) {
    let game_menu = hotkeys.register(
        Event::new()
            .trigger(DEFAULT_TRIGGER_GAMEMENU)
            .kind(TriggerKind::Trigger),
    );

    unsafe {
        EVENTS.write(Events { game_menu });
    }
}

pub(super) fn handle_events(mut hotkeys: Res<HotkeyStore>, mut state: ResMut<InterfaceState>) {
    let events = unsafe { EVENTS.assume_init_ref() };

    if hotkeys.triggered(events.game_menu) {
        state.toggle(MENU_GAME);
    }
}
