use std::collections::VecDeque;
use std::sync::mpsc;

use glam::Vec2;
use parking_lot::RwLock;

use crate::windows::{UpdateEvent, WindowId};
use crate::Backend;

pub type CursorIcon = winit::window::CursorIcon;

#[derive(Debug)]
pub struct Cursor {
    pub(crate) state: RwLock<CursorState>,
    tx: mpsc::Sender<UpdateEvent>,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct CursorState {
    pub is_locked: bool,
    pub window: Option<WindowId>,
    pub position: Vec2,
}

impl Cursor {
    pub(crate) fn new(tx: mpsc::Sender<UpdateEvent>) -> Self {
        Self {
            state: RwLock::new(CursorState {
                is_locked: false,
                window: None,
                position: Vec2::splat(0.0),
            }),
            tx,
        }
    }

    #[inline]
    pub fn window(&self) -> Option<WindowId> {
        let state = self.state.read();
        state.window
    }

    #[inline]
    pub fn position(&self) -> Vec2 {
        let state = self.state.read();
        state.position
    }

    pub fn is_locked(&self) -> bool {
        let state = self.state.read();
        state.is_locked
    }

    #[inline]
    pub fn lock(&mut self) {
        let state = self.state.read();

        if state.is_locked {
            return;
        }

        let Some(window) = state.window else {
            return;
        };

        let _ = self
            .tx
            .send(UpdateEvent::CursorGrab(window, CursorGrabMode::Locked));
    }

    #[inline]
    pub fn unlock(&mut self) {
        let state = self.state.read();

        if !state.is_locked {
            return;
        }

        let Some(window) = state.window else {
            return;
        };

        let _ = self
            .tx
            .send(UpdateEvent::CursorGrab(window, CursorGrabMode::None));
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum CursorGrabMode {
    #[default]
    None,
    Locked,
}

/// Cross-Platform compatability support
#[derive(Debug)]
pub(crate) struct WindowCompat {
    backend: Backend,
    cursor_grab_mode: CursorGrabMode,
    /// Cursor position should be reset.
    reset_cursor_position: bool,
}

impl WindowCompat {
    pub fn new(backend: Backend) -> Self {
        Self {
            backend,
            cursor_grab_mode: CursorGrabMode::None,
            reset_cursor_position: false,
        }
    }

    pub fn move_cursor(&mut self) {
        if self.backend.supports_locked_cursor() || self.cursor_grab_mode != CursorGrabMode::Locked
        {
            return;
        }

        self.reset_cursor_position = true;
    }

    pub fn emulate_cursor_grab_mode_locked(
        &self,
        cursor: &Cursor,
        events: &mut VecDeque<UpdateEvent>,
    ) {
        if !self.reset_cursor_position {
            return;
        }

        if let Some(id) = cursor.window() {
            events.push_back(UpdateEvent::CursorPosition(id, cursor.position()));
        }
    }
}
