//! Wayland clipboard implementation
//!

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use game_window::windows::{WindowId, WindowState};
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};

/// The backend of a Wayland clipboard.
// Note that the clipboad on wayland works different:
// Instead of the there being a global clipboard instance
// every wayland "display" has its own clipboard. On most
// systems there is only one such "display".
// However to know the value of the display handle we must
// first have a window and that window must stay alive
// until the clipboard has been dropped.
#[derive(Debug)]
pub(super) struct WaylandBackend {
    clipboards: HashMap<usize, WaylandClipboard>,
    windows: HashMap<WindowId, WaylandWindow>,
}

impl WaylandBackend {
    pub(super) fn new() -> Self {
        Self {
            clipboards: HashMap::new(),
            windows: HashMap::new(),
        }
    }

    pub(super) fn create(&mut self, window: WindowState) {
        let display_handle = match window.display_handle().map(|v| v.as_raw()) {
            Ok(RawDisplayHandle::Wayland(display_handle)) => display_handle,
            _ => return,
        };

        self.clipboards
            .entry(display_handle.display.as_ptr() as usize)
            .or_insert_with(|| {
                // SAFETY: To guarantee that the display handle becomes invalid we
                // keep a local copy of the `WindowState` that owns the handle.
                let clipboard =
                    unsafe { smithay_clipboard::Clipboard::new(display_handle.display.as_ptr()) };

                WaylandClipboard {
                    clipboard,
                    window_count: 0,
                }
            })
            .window_count += 1;

        self.windows.insert(
            window.id(),
            WaylandWindow {
                display_handle: display_handle.display.as_ptr() as usize,
                _window: window,
            },
        );
    }

    pub(super) fn destroy(&mut self, window: WindowId) {
        let Some(window) = self.windows.remove(&window) else {
            return;
        };

        let clipboard = self.clipboards.get_mut(&window.display_handle).unwrap();
        clipboard.window_count -= 1;

        if clipboard.window_count == 0 {
            self.clipboards.remove(&window.display_handle);
        }
    }

    /// Returns the clipboard for a specific window, if any.
    pub(super) fn clipboard(&self, window: WindowId) -> Option<&WaylandClipboard> {
        let window = self.windows.get(&window)?;
        self.clipboards.get(&window.display_handle)
    }
}

/// A Wayland window.
#[derive(Debug)]
struct WaylandWindow {
    display_handle: usize,
    _window: WindowState,
}

/// A Wayland clipboard, bound to a display.
pub(super) struct WaylandClipboard {
    /// The underlying clipboard.
    clipboard: smithay_clipboard::Clipboard,
    /// The number of [`WaylandWindow`]s pointing at this `WaylandClipboard`.
    window_count: usize,
}

impl WaylandClipboard {
    pub(super) fn get(&self) -> Option<String> {
        self.clipboard.load().ok()
    }

    pub(super) fn set(&self, value: &str) {
        self.clipboard.store(value);
    }
}

impl Debug for WaylandClipboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WaylandClipboard")
            .field("window_count", &self.window_count)
            .finish_non_exhaustive()
    }
}
