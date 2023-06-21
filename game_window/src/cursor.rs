use std::collections::VecDeque;
use std::sync::Arc;

use bevy_ecs::prelude::{Entity, EventReader};
use bevy_ecs::system::{Query, ResMut, Resource};
use glam::Vec2;
use parking_lot::Mutex;
use winit::window::CursorGrabMode;

use crate::events::{CursorLeft, CursorMoved};
use crate::WindowState;

pub type CursorIcon = winit::window::CursorIcon;

#[derive(Clone, Debug, Default, Resource)]
pub struct Cursor {
    is_locked: bool,
    /// The window that the cursor is located on.
    window: Option<Entity>,
    position: Vec2,
    queue: Arc<Mutex<VecDeque<(Entity, CursorEvent)>>>,
}

impl Cursor {
    #[inline]
    pub fn new() -> Self {
        Self {
            is_locked: false,
            window: None,
            position: Vec2::splat(0.0),
            queue: Arc::default(),
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

    pub fn is_locked(&self) -> bool {
        self.is_locked
    }

    #[inline]
    pub fn lock(&mut self) {
        if !self.is_locked {
            self.lock_unchecked();
        }
    }

    #[inline]
    pub fn unlock(&mut self) {
        if self.is_locked {
            self.unlock_unchecked();
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        if self.is_locked {
            let entity = self.window.unwrap();

            self.queue
                .lock()
                .push_back((entity, CursorEvent::Position(self.position)));
        }
    }

    /// Locks the `Cursor` to its current position without checking if it is already locked.
    ///
    /// Consider using [`lock`] if the cursor may already be locked.
    ///
    /// [`lock`]: Self::lock
    #[inline]
    pub fn lock_unchecked(&mut self) {
        let entity = self.window.unwrap();

        self.queue
            .lock()
            .push_back((entity, CursorEvent::CursorGrab(CursorGrabMode::Locked)));
        self.queue
            .lock()
            .push_back((entity, CursorEvent::CursorVisible(false)));

        self.is_locked = true;
    }

    #[inline]
    pub fn unlock_unchecked(&mut self) {
        let entity = self.window.unwrap();

        self.queue
            .lock()
            .push_back((entity, CursorEvent::CursorGrab(CursorGrabMode::None)));
        self.queue
            .lock()
            .push_back((entity, CursorEvent::CursorVisible(true)));

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

pub fn flush_cursor_events(mut cursor: ResMut<Cursor>, windows: Query<&WindowState>) {
    while let Some((entity, event)) = cursor.queue.lock().pop_front() {
        let window = windows.get(entity).unwrap();

        match event {
            CursorEvent::CursorGrab(mode) => {
                tracing::debug!("setting cursor grab mode to {:?}", mode);

                if let Err(err) = window.set_cursor_grab(mode) {
                    tracing::error!("failed to set cursor grab mode: {}", err);
                }
            }
            CursorEvent::CursorVisible(visible) => {
                tracing::debug!("setting cursor visibility to {:?}", visible);

                window.set_cursor_visibility(visible);
            }
            CursorEvent::Position(position) => {
                tracing::debug!("setting cursor position to {:?}", position);

                if let Err(err) = window.set_cursor_position(position) {
                    tracing::error!("failed to set cursor position: {}", err);
                }
            }
        }
    }
}

#[derive(Debug)]
enum CursorEvent {
    CursorGrab(CursorGrabMode),
    CursorVisible(bool),
    Position(Vec2),
}
