mod console;
mod crosshair;
mod death;
mod gamemenu;
mod health;
mod inventory;

use std::borrow::Cow;

use bevy::prelude::{App, Input, KeyCode, Res, ResMut};
pub use console::Console;
pub use crosshair::Crosshair;
pub use death::Death;
pub use gamemenu::GameMenu;
pub use health::Health;
pub use inventory::Inventory;

use input::hotkeys::{
    Hotkey, HotkeyCode, HotkeyFilter, HotkeyId, HotkeyReader, Hotkeys, Key, TriggerKind,
};

use crate::InterfaceState;

static mut INVENTORY: Hotkey = Hotkey {
    id: HotkeyId(0),
    name: Cow::Borrowed("inventory"),
    default: Key {
        trigger: TriggerKind::JustPressed,
        code: HotkeyCode::KeyCode {
            key_code: KeyCode::I,
        },
    },
};

struct InventoryHotkey;

impl HotkeyFilter for InventoryHotkey {
    fn filter(id: HotkeyId) -> bool {
        let want = unsafe { &INVENTORY }.id;
        want == id
    }
}

pub(super) fn register_hotkeys(mut hotkeys: ResMut<Hotkeys>) {
    let mut inventory = unsafe { &mut INVENTORY };
    let id = hotkeys.register(inventory.clone());
    inventory.id = id;

    drop(inventory);
}

pub(super) fn register_hotkey_systems(app: &mut App) {
    app.add_system(escape).add_system(toggle_inventory);
}

fn escape(mut state: ResMut<InterfaceState>, inputs: Res<Input<KeyCode>>) {
    if !inputs.just_pressed(KeyCode::Escape) {
        return;
    }

    if !state.pop() {
        state.push(GameMenu::default());
    }
}

fn toggle_inventory(mut state: ResMut<InterfaceState>, mut events: HotkeyReader<InventoryHotkey>) {
    for _ in events.iter() {
        state.push(Inventory::default());
    }
}
