use bevy::prelude::{Resource, Vec2};
use bevy::window::{CursorGrabMode, Window};

#[derive(Copy, Clone, Debug, Default, PartialEq, Resource)]
pub struct Cursor(Option<Vec2>);

impl Cursor {
    #[inline]
    pub const fn new() -> Self {
        Self(None)
    }

    #[inline]
    pub fn lock(&mut self, window: &mut Window) {
        window.set_cursor_visibility(false);
        window.set_cursor_grab_mode(CursorGrabMode::Locked);

        self.0 = window.cursor_position();
    }

    #[inline]
    pub fn unlock(&mut self, window: &mut Window) {
        window.set_cursor_visibility(true);
        window.set_cursor_grab_mode(CursorGrabMode::None);

        self.0 = None;
    }

    #[inline]
    pub fn reset(&self, window: &mut Window) {
        if let Some(position) = self.0 {
            window.set_cursor_position(position);
        }
    }
}
