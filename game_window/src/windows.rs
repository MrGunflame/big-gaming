use std::borrow::Cow;
use std::sync::{mpsc, Arc};

use game_common::collections::arena::{Arena, Key};
use glam::{UVec2, Vec2};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use winit::dpi::{LogicalPosition, Position};
use winit::error::ExternalError;

use crate::cursor::{CursorGrabMode, CursorIcon};
use crate::Backend;

const DEFAULT_TITLE: &str = "DEFAULT_TITLE";

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub(crate) Key);

// We don't provide a `Clone` impl to prevent accidently
// cloning and leaking the window state, which would cause
// a deadlock.
#[derive(Debug)]
pub struct Windows {
    pub(crate) windows: Arena<Window>,
    tx: mpsc::Sender<UpdateEvent>,
}

impl Windows {
    pub(crate) fn new(tx: mpsc::Sender<UpdateEvent>) -> Self {
        Self {
            windows: Arena::new(),
            tx,
        }
    }

    /// Spawns a new [`Window`].
    pub fn spawn<T>(&mut self, window: T) -> WindowId
    where
        T: Into<Window>,
    {
        let window = window.into();

        let key = self.windows.insert(window);

        let _ = self.tx.send(UpdateEvent::Create(WindowId(key)));
        WindowId(key)
    }

    /// Despawns the window with the given `id`.
    pub fn despawn(&mut self, id: WindowId) {
        self.windows.remove(id.0);

        let _ = self.tx.send(UpdateEvent::Destroy(id));
    }

    pub fn state(&self, id: WindowId) -> Option<WindowState> {
        self.windows
            .get(id.0)
            .and_then(|window| window.state.clone())
    }

    pub fn get(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(id.0)
    }

    pub(crate) fn get_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(id.0)
    }

    pub(crate) fn remove_any(&mut self) -> Option<(WindowId, Window)> {
        let id = self.windows.keys().next()?;
        let window = self.windows.remove(id).unwrap();
        Some((WindowId(id), window))
    }
}

#[derive(Clone, Debug)]
pub struct WindowBuilder {
    title: Cow<'static, str>,
}

impl WindowBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            title: Cow::Borrowed(DEFAULT_TITLE),
        }
    }

    /// Sets the title of the window.
    #[inline]
    pub fn title<T>(mut self, title: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.title = title.into();
        self
    }
}

impl Default for WindowBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl From<WindowBuilder> for Window {
    fn from(builder: WindowBuilder) -> Self {
        Self {
            title: builder.title,
            state: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Window {
    pub(crate) title: Cow<'static, str>,
    pub(crate) state: Option<WindowState>,
}

#[derive(Clone, Debug)]
pub struct WindowState {
    pub(crate) id: WindowId,
    // Note: It is important that the window handle itself sits
    // behind an Arc and is not immediately dropped once the window
    // component is despawned. Rendering surfaces require the handle
    // to be valid until the surface was dropped in the rendering
    // crate.
    pub(crate) inner: Arc<winit::window::Window>,
    pub(crate) backend: Backend,
}

impl WindowState {
    #[inline]
    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn backend(&self) -> Backend {
        self.backend
    }

    pub fn inner_size(&self) -> UVec2 {
        let size = self.inner.inner_size();
        UVec2 {
            x: size.width,
            y: size.height,
        }
    }

    pub fn scale_factor(&self) -> f64 {
        self.inner.scale_factor()
    }

    /// Sets the position of the cursor within this `Window`.
    ///
    /// # Errors
    ///
    /// Returns an error if setting the position fails.
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

    /// Sets the [`CursorGrabMode`] of this `Window`.
    ///
    /// # Errors
    ///
    /// Returns an error if setting the [`CursorGrabMode`] fails.
    pub fn set_cursor_grab(&self, mode: CursorGrabMode) -> Result<(), ExternalError> {
        let mode = match mode {
            CursorGrabMode::None => winit::window::CursorGrabMode::None,
            CursorGrabMode::Locked => {
                // Some backends on't support `Locked`, we must set it to
                // `Confined` and constantly reset the cursor to the origin.
                if self.backend.supports_locked_cursor() {
                    winit::window::CursorGrabMode::Locked
                } else {
                    winit::window::CursorGrabMode::Confined
                }
            }
        };

        self.inner.set_cursor_grab(mode)
    }

    /// Sets the title of the `Window`.
    pub fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    /// Sets the [`CursorIcon`] of the `Window`.
    pub fn set_cursor_icon(&self, icon: CursorIcon) {
        self.inner.set_cursor_icon(icon)
    }

    pub fn pre_present_notify(&self) {
        self.inner.pre_present_notify();
    }
}

impl HasDisplayHandle for WindowState {
    #[inline]
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}

impl HasWindowHandle for WindowState {
    #[inline]
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}

pub(crate) enum UpdateEvent {
    Create(WindowId),
    Destroy(WindowId),
    CursorGrab(WindowId, CursorGrabMode),
    CursorVisible(WindowId, bool),
    CursorPosition(WindowId, Vec2),
}
