use bevy::window::{CursorGrabMode, Window};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Cursor;

impl Cursor {
    pub fn lock(window: &mut Window) {
        window.set_cursor_visibility(false);
        window.set_cursor_grab_mode(CursorGrabMode::Locked);

        if let Some(pos) = window.cursor_position() {
            window.set_cursor_position(pos);
        }
    }

    pub fn unlock(window: &mut Window) {
        window.set_cursor_visibility(true);
        window.set_cursor_grab_mode(CursorGrabMode::None);
    }
}
