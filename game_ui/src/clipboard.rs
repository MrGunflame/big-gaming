use std::fmt::{self, Debug, Formatter};

use game_tracing::trace_span;

pub(crate) struct Clipboard {
    inner: Option<arboard::Clipboard>,
}

impl Clipboard {
    pub fn new() -> Self {
        let inner = match arboard::Clipboard::new() {
            Ok(clipboard) => Some(clipboard),
            Err(err) => {
                tracing::error!("failed to open clipboard: {}", err);
                tracing::warn!("clipboard operations will not available");
                None
            }
        };

        Self { inner }
    }

    pub fn get(&mut self) -> Option<String> {
        let _span = trace_span!("Clipboard::get").entered();

        self.inner
            .as_mut()
            .map(|inner| inner.get_text().ok())
            .flatten()
    }

    pub fn set(&mut self, value: &str) {
        let _span = trace_span!("Clipboard::set").entered();

        if let Some(inner) = &mut self.inner {
            inner.set_text(value).ok();
        }
    }
}

impl Debug for Clipboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Clipboard").finish_non_exhaustive()
    }
}
