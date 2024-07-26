mod button;
mod svg;
// mod checkbox;
mod container;
mod image;
mod input;
// mod parse_input;
mod context_menu;
mod plot;
mod selection;
mod table;
mod text;

pub use self::image::Image;
pub use button::Button;
pub use container::Container;
pub use context_menu::{ContextMenuCloser, ContextMenuState, ContextPanel};
use game_tracing::trace_span;
pub use input::Input;
use parking_lot::Mutex;
pub use plot::Plot;
pub use selection::Selection;
pub use svg::{Svg, SvgData, SvgError, SvgStyle};
pub use table::{Table, TableStyle};
pub use text::Text;

use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use crate::reactive::Context;

pub trait Widget {
    fn mount<T>(self, parent: &Context<T>) -> Context<()>;
}

pub struct Callback<T>(Option<Arc<Mutex<dyn FnMut(T) + Send + Sync + 'static>>>);

impl<T> Callback<T> {
    pub fn call(&self, value: T) {
        let _span = trace_span!("Callback::call").entered();

        if let Some(f) = &self.0 {
            (f.try_lock().unwrap())(value);
        }
    }
}

impl<T> Default for Callback<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T, F> From<F> for Callback<T>
where
    F: FnMut(T) + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        Self(Some(Arc::new(Mutex::new(value))))
    }
}

impl<T> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Debug for Callback<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callback")
            .field("ptr", &self.0.as_ref().map(|arc| Arc::as_ptr(&arc)))
            .finish()
    }
}
