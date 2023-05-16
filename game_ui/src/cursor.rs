use bevy_ecs::prelude::{Entity, EventReader};
use bevy_ecs::system::{ResMut, Resource};
use game_window::events::{CursorLeft, CursorMoved};
use game_window::WindowState;
use glam::Vec2;
use winit::window::CursorGrabMode;

// FIXME: This should probably be in another crate, input?
#[derive(Copy, Clone, Debug, Default, PartialEq, Resource)]
pub struct Cursor {
    is_locked: bool,
    /// The window that the cursor is located on.
    window: Option<Entity>,
    position: Vec2,
}

impl Cursor {
    #[inline]
    pub const fn new() -> Self {
        Self {
            is_locked: false,
            window: None,
            position: Vec2::splat(0.0),
        }
    }

    #[inline]
    pub fn window(&self) -> Option<Entity> {
        self.window
    }

    #[inline]
    pub fn position(&self) -> Vec2 {
        self.position
    }

    #[inline]
    pub fn lock(&mut self, window: &mut WindowState) {
        if !self.is_locked {
            self.lock_unchecked(window);
        }
    }

    #[inline]
    pub fn unlock(&mut self, window: &mut WindowState) {
        if self.is_locked {
            self.unlock_unchecked(window);
        }
    }

    #[inline]
    pub fn reset(&self, window: &mut WindowState) {
        if self.is_locked {
            window.set_cursor_position(self.position);
        }
    }

    /// Locks the `Cursor` to its current position without checking if it is already locked.
    ///
    /// Consider using [`lock`] if the cursor may already be locked.
    ///
    /// [`lock`]: Self::lock
    #[inline]
    pub fn lock_unchecked(&mut self, window: &mut WindowState) {
        window.set_cursor_visibility(false);
        window.set_cursor_grab(CursorGrabMode::Locked);

        self.is_locked = true;
        // self.position = window.cursor_position().unwrap_or_default();
    }

    #[inline]
    pub fn unlock_unchecked(&mut self, window: &mut WindowState) {
        window.set_cursor_visibility(true);
        window.set_cursor_grab(CursorGrabMode::None);

        self.is_locked = false;
    }
}

pub fn update_cursor_position(
    mut cursor: ResMut<Cursor>,
    mut moved: EventReader<CursorMoved>,
    mut left: EventReader<CursorLeft>,
) {
    if cursor.is_locked {
        moved.clear();
        left.clear();
        return;
    }

    for event in moved.iter() {
        cursor.window = Some(event.window);
        cursor.position = event.position;
    }

    for event in left.iter() {
        cursor.window = None;
    }
}
