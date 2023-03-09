use bevy::prelude::{Resource, Vec2};
use bevy::window::{CursorGrabMode, Window};

#[derive(Copy, Clone, Debug, Default, PartialEq, Resource)]
pub struct Cursor {
    is_locked: bool,
    position: Vec2,
}

impl Cursor {
    #[inline]
    pub const fn new() -> Self {
        Self {
            is_locked: false,
            position: Vec2::splat(0.0),
        }
    }

    #[inline]
    pub fn lock(&mut self, window: &mut Window) {
        if !self.is_locked {
            self.lock_unchecked(window);
        }
    }

    #[inline]
    pub fn unlock(&mut self, window: &mut Window) {
        if self.is_locked {
            self.unlock_unchecked(window);
        }
    }

    #[inline]
    pub fn reset(&self, window: &mut Window) {
        if self.is_locked {
            window.set_cursor_position(Some(self.position));
        }
    }

    /// Locks the `Cursor` to its current position without checking if it is already locked.
    ///
    /// Consider using [`lock`] if the cursor may already be locked.
    ///
    /// [`lock`]: Self::lock
    #[inline]
    pub fn lock_unchecked(&mut self, window: &mut Window) {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;

        self.is_locked = true;
        self.position = window.cursor_position().unwrap_or_default();
    }

    #[inline]
    pub fn unlock_unchecked(&mut self, window: &mut Window) {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;

        self.is_locked = false;
    }
}
