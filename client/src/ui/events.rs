use std::mem::MaybeUninit;

use bevy::prelude::{EventReader, EventWriter, KeyCode, Query, Res, ResMut};
use bevy::window::Windows;

use crate::plugins::hotkeys::{Event, EventId, HotkeyStore, TriggerKind};

use super::cursor::Cursor;
use super::interfaces::{MENU_DEATH, MENU_DEBUG, MENU_GAME};
use super::{menu, Focus, InterfaceState};

const DEFAULT_TRIGGER_GAMEMENU: KeyCode = KeyCode::Escape;
const DEFAULT_TRIGGER_DEBUGMENU: KeyCode = KeyCode::F3;

static mut EVENTS: MaybeUninit<Events> = MaybeUninit::uninit();

struct Events {
    game_menu: EventId,
    debug_menu: EventId,
}

pub(super) fn register_events(mut hotkeys: ResMut<HotkeyStore>) {
    let game_menu = hotkeys.register(
        Event::new()
            .trigger(DEFAULT_TRIGGER_GAMEMENU)
            .kind(TriggerKind::Trigger),
    );

    let debug_menu = hotkeys.register(
        Event::new()
            .trigger(DEFAULT_TRIGGER_DEBUGMENU)
            .kind(TriggerKind::Trigger),
    );

    unsafe {
        EVENTS.write(Events {
            game_menu,
            debug_menu,
        });
    }
}

pub(super) fn handle_events(
    hotkeys: Res<HotkeyStore>,
    mut state: ResMut<InterfaceState>,
    mut focus: EventWriter<Focus>,
) {
    let events = unsafe { EVENTS.assume_init_ref() };

    if hotkeys.triggered(events.game_menu) {
        if state.contains(MENU_GAME) {
            unsafe {
                state.remove::<_, menu::gamemenu::State>(MENU_GAME);
            }

            focus.send(Focus::World);
        } else {
            state.insert(MENU_GAME, Some(menu::gamemenu::State::default()));

            focus.send(Focus::Interface);
        }
    }

    if hotkeys.triggered(events.debug_menu) {
        if state.contains(MENU_DEBUG) {
            unsafe {
                state.remove::<_, ()>(MENU_DEBUG);
            }
        } else {
            state.insert::<()>(MENU_DEBUG, None);
        }
    }
}

/// Toggle [`Focus`].
pub(super) fn toggle_focus(
    mut windows: ResMut<Windows>,
    mut players: Query<&mut Focus>,
    mut events: EventReader<Focus>,
) {
    let window = windows.primary_mut();
    let mut focus = players.single_mut();

    for event in events.iter() {
        *focus = *event;

        match event {
            Focus::World => Cursor::lock(window),
            Focus::Interface => Cursor::unlock(window),
        }
    }
}
