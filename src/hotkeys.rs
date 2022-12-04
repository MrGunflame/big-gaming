use bevy::prelude::{Input, KeyCode, Resource};

#[derive(Clone, Debug, Resource)]
pub struct HotkeyStore {
    // Movement
    pub move_foward: KeyCode,
    pub move_backwards: KeyCode,
    pub move_left: KeyCode,
    pub move_right: KeyCode,
}

impl HotkeyStore {
    /// Returns `true` if a [`Hotkey`] is currently being pressed.
    pub fn pressed<T>(&self, input: &Input<KeyCode>) -> bool
    where
        T: Hotkey,
    {
        T::pressed(self, input)
    }
}

impl Default for HotkeyStore {
    fn default() -> Self {
        Self {
            move_foward: KeyCode::W,
            move_backwards: KeyCode::S,
            move_left: KeyCode::A,
            move_right: KeyCode::D,
        }
    }
}

pub trait Hotkey {
    fn pressed(store: &HotkeyStore, input: &Input<KeyCode>) -> bool;
}

/// The [`Hotkey`] for moving forwards.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MoveForward;

impl Hotkey for MoveForward {
    fn pressed(store: &HotkeyStore, input: &Input<KeyCode>) -> bool {
        input.pressed(store.move_foward)
    }
}

/// The [`Hotkey`] for moving backwards.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MoveBackward;

impl Hotkey for MoveBackward {
    fn pressed(store: &HotkeyStore, input: &Input<KeyCode>) -> bool {
        input.pressed(store.move_backwards)
    }
}

/// The [`Hotkey`] for moving left.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MoveLeft;

impl Hotkey for MoveLeft {
    fn pressed(store: &HotkeyStore, input: &Input<KeyCode>) -> bool {
        input.pressed(store.move_left)
    }
}

/// The [`Hotkey`] for moving right.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MoveRight;

impl Hotkey for MoveRight {
    fn pressed(store: &HotkeyStore, input: &Input<KeyCode>) -> bool {
        input.pressed(store.move_right)
    }
}
