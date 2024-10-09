#[cfg(unix)]
mod wayland;

use std::fmt::{self, Debug, Formatter};

use game_tracing::trace_span;
use game_window::windows::{WindowId, WindowState};

#[cfg(unix)]
use wayland::WaylandBackend;

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
            if window.backend().is_wayland() {
                #[cfg(unix)]
                {
                    self.backend = Backend::Wayland(WaylandBackend::new())
                }
            } else {
                self.backend = match arboard::Clipboard::new() {
                    Ok(clipboard) => Backend::Arboard(clipboard),
                    Err(err) => {
                        tracing::error!("failed to open clipboard: {}", err);
                        tracing::warn!("clipboard operations will not available");
                        Backend::None
                    }
                };
            }
        }

        #[cfg(unix)]
        if let Backend::Wayland(backend) = &mut self.backend {
            backend.create(window);
        }
    }

    pub(crate) fn destroy(&mut self, window: WindowId) {
        let _span = trace_span!("Backend::destroy").entered();

        #[cfg(unix)]
        if let Backend::Wayland(backend) = &mut self.backend {
            backend.destroy(window);
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
            #[cfg(unix)]
            Backend::Wayland(backend) => backend.clipboard(window)?.get(),
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
            #[cfg(unix)]
            Backend::Wayland(backend) => {
                if let Some(clipboard) = backend.clipboard(window) {
                    clipboard.set(value);
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
    #[cfg(unix)]
    Wayland(WaylandBackend),
}
