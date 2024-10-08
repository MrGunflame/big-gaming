use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use game_tracing::trace_span;
use game_window::windows::{WindowId, WindowState};
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};

pub(crate) struct Clipboard {
    backend: Backend,
}

impl Clipboard {
    pub(crate) fn new() -> Self {
        Self {
            backend: Backend::NotInit,
        }
    }

    pub(crate) fn create(&mut self, window: WindowState) {
        let _span = trace_span!("Clipboard::create").entered();

        if matches!(self.backend, Backend::NotInit) {
            self.backend = if window.backend().is_wayland() {
                Backend::Wayland(WaylandBackend {
                    clipboards: HashMap::new(),
                    windows: HashMap::new(),
                })
            } else {
                match arboard::Clipboard::new() {
                    Ok(clipboard) => Backend::Arboard(clipboard),
                    Err(err) => {
                        tracing::error!("failed to open clipboard: {}", err);
                        tracing::warn!("clipboard operations will not available");
                        Backend::None
                    }
                }
            };
        }

        // Note that the clipboad on wayland works different:
        // Instead of the there being a global clipboard instance
        // every wayland "display" has its own clipboard. On most
        // systems there is only one such "display".
        // However to know the value of the display handle we must
        // first have a window and that window must stay alive
        // until the clipboard has been dropped.
        if let Backend::Wayland(backend) = &mut self.backend {
            let display_handle = match window.display_handle().map(|v| v.as_raw()) {
                Ok(RawDisplayHandle::Wayland(display_handle)) => display_handle,
                _ => return,
            };

            backend
                .clipboards
                .entry(display_handle.display.as_ptr() as usize)
                .or_insert_with(|| {
                    // SAFETY: To guarantee that the display handle becomes invalid we
                    // keep a local copy of the `WindowState` that owns the handle.
                    let clipboard = unsafe {
                        smithay_clipboard::Clipboard::new(display_handle.display.as_ptr())
                    };

                    WaylandClipboard {
                        clipboard,
                        window_count: 0,
                    }
                })
                .window_count += 1;

            backend.windows.insert(
                window.id(),
                WaylandWindow {
                    display_handle: display_handle.display.as_ptr() as usize,
                    _window: window,
                },
            );
        }
    }

    pub(crate) fn destroy(&mut self, window: WindowId) {
        let _span = trace_span!("Backend::destroy").entered();

        if let Backend::Wayland(backend) = &mut self.backend {
            let Some(window) = backend.windows.remove(&window) else {
                return;
            };

            let clipboard = backend.clipboards.get_mut(&window.display_handle).unwrap();
            clipboard.window_count -= 1;

            if clipboard.window_count == 0 {
                backend.clipboards.remove(&window.display_handle);
            }
        }
    }

    /// Returns the contents of the clipboard in the given `window`.
    ///
    /// Returns `None` if the clipboard is empty, or an error occured.
    pub(crate) fn get(&mut self, window: WindowId) -> Option<String> {
        let _span = trace_span!("Clipboard::get").entered();

        match &mut self.backend {
            Backend::NotInit | Backend::None => None,
            Backend::Arboard(backend) => backend.get_text().ok(),
            Backend::Wayland(backend) => backend.clipboard(window)?.clipboard.load().ok(),
        }
    }

    /// Sets contents of the clipboard in the given `window`.
    pub(crate) fn set(&mut self, window: WindowId, value: &str) {
        let _span = trace_span!("Clipboard::set").entered();

        match &mut self.backend {
            Backend::NotInit | Backend::None => (),
            Backend::Arboard(backend) => {
                backend.set_text(value).ok();
            }
            Backend::Wayland(backend) => {
                if let Some(clipboard) = backend.clipboard(window) {
                    clipboard.clipboard.store(value);
                }
            }
        }
    }
}

impl Debug for Clipboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Clipboard").finish_non_exhaustive()
    }
}

enum Backend {
    NotInit,
    None,
    Arboard(arboard::Clipboard),
    Wayland(WaylandBackend),
}

#[derive(Debug)]
struct WaylandBackend {
    clipboards: HashMap<usize, WaylandClipboard>,
    windows: HashMap<WindowId, WaylandWindow>,
}

impl WaylandBackend {
    /// Returns the clipboard for a specific window, if any.
    fn clipboard(&self, window: WindowId) -> Option<&WaylandClipboard> {
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
struct WaylandClipboard {
    /// The underlying clipboard.
    clipboard: smithay_clipboard::Clipboard,
    /// The number of [`WaylandWindow`]s pointing at this `WaylandClipboard`.
    window_count: usize,
}

impl Debug for WaylandClipboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WaylandClipboard")
            .field("window_count", &self.window_count)
            .finish_non_exhaustive()
    }
}
