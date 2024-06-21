mod button;
// mod checkbox;
mod container;
// mod image;
mod input;
// mod parse_input;
// mod plot;
mod selection;
mod table;
mod text;

// pub mod value_slider;

// pub use self::image::Image;
pub use button::Button;
// pub use checkbox::Checkbox;
pub use container::Container;
pub use input::Input;
use parking_lot::Mutex;
// pub use parse_input::ParseInput;
// pub use plot::Plot;
pub use selection::Selection;
pub use table::Table;
pub use text::Text;

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use crate::reactive::Context;

pub trait Widget {
    fn mount<T>(self, parent: &Context<T>) -> Context<()>;
}

pub struct Callback<T>(Option<Arc<Mutex<dyn FnMut(T) + Send + Sync + 'static>>>);

impl<T> Callback<T> {
    pub fn call(&self, value: T) {
        if let Some(f) = &self.0 {
            (f.lock())(value);
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
