use std::sync::{mpsc, Arc};

use glam::{UVec2, Vec2};
use parking_lot::RwLock;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use slotmap::{DefaultKey, SlotMap};
use winit::dpi::{LogicalPosition, PhysicalSize, Position};
use winit::error::ExternalError;

use crate::cursor::{CursorGrabMode, CursorIcon};
use crate::Backend;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub(crate) DefaultKey);

#[derive(Clone, Debug)]
pub struct Windows {
    pub(crate) windows: Arc<RwLock<SlotMap<DefaultKey, Window>>>,
    tx: mpsc::Sender<UpdateEvent>,
}

impl Windows {
    pub(crate) fn new(tx: mpsc::Sender<UpdateEvent>) -> Self {
        Self {
            windows: Arc::new(RwLock::new(SlotMap::new())),
            tx,
        }
    }

    pub fn spawn<T>(&self, window: T) -> WindowId
    where
        T: Into<Window>,
    {
        let mut windows = self.windows.write();
        let key = windows.insert(window.into());

        let _ = self.tx.send(UpdateEvent::Create(WindowId(key)));
        WindowId(key)
    }

    pub fn state(&self, id: WindowId) -> Option<WindowState> {
        let windows = self.windows.read();
        windows
            .get(id.0)
            .map(|window| window.state.clone())
            .flatten()
    }
}

#[derive(Clone, Debug)]
pub struct WindowBuilder {}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<WindowBuilder> for Window {
    fn from(value: WindowBuilder) -> Self {
        Self { state: None }
    }
}

#[derive(Clone, Debug)]
pub struct Window {
    pub(crate) state: Option<WindowState>,
}

#[derive(Clone, Debug)]
pub struct WindowState {
    // Note: It is important that the window handle itself sits
    // behind an Arc and is not immediately dropped once the window
    // component is despawned. Rendering surfaces require the handle
    // to be valid until the surface was dropped in the rendering
    // crate.
    pub(crate) inner: Arc<winit::window::Window>,
    pub(crate) backend: Backend,
}

impl WindowState {
    pub fn inner_size(&self) -> UVec2 {
        let size = self.inner.inner_size();
        UVec2 {
            x: size.width,
            y: size.height,
        }
    }

    pub fn set_cursor_position(&self, position: Vec2) -> Result<(), ExternalError> {
        self.inner
            .set_cursor_position(Position::Logical(LogicalPosition {
                x: position.x as f64,
                y: position.y as f64,
            }))
    }

    pub fn set_cursor_visibility(&self, visible: bool) {
        self.inner.set_cursor_visible(visible);
    }

    pub fn set_cursor_grab(&self, mode: CursorGrabMode) -> Result<(), ExternalError> {
        let mode = match mode {
            CursorGrabMode::None => winit::window::CursorGrabMode::None,
            CursorGrabMode::Locked => match self.backend {
                Backend::Wayland | Backend::Unknown => winit::window::CursorGrabMode::Locked,
                // X11 and Windows don't support `Locked`, we must set it to
                // `Confined` and constantly reset the cursor to the origin.
                Backend::X11 | Backend::Windows => winit::window::CursorGrabMode::Confined,
            },
        };

        self.inner.set_cursor_grab(mode)
    }

    pub fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    pub fn set_cursor_icon(&self, icon: CursorIcon) {
        self.inner.set_cursor_icon(icon)
    }

    pub(crate) fn backend(&self) -> Backend {
        self.backend
    }
}

unsafe impl HasRawDisplayHandle for WindowState {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.inner.raw_display_handle()
    }
}

unsafe impl HasRawWindowHandle for WindowState {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.inner.raw_window_handle()
    }
}

pub(crate) enum UpdateEvent {
    Create(WindowId),
    CursorGrab(WindowId, CursorGrabMode),
    CursorVisible(WindowId, bool),
    CursorPosition(WindowId, Vec2),
}
