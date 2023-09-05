mod button;
mod checkbox;
mod container;
mod image;
mod input;
mod parse_input;
mod selection;
mod text;

pub use self::image::Image;
pub use button::Button;
pub use checkbox::Checkbox;
pub use container::Container;
pub use input::Input;
pub use parse_input::ParseInput;
pub use selection::{Selection, SelectionProps};
pub use text::Text;

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use crate::reactive::{ReadSignal, Scope};

pub trait Widget {
    fn build(self, cx: &Scope) -> Scope;
}

pub struct Callback<I>(pub Arc<dyn Fn(I) + Send + Sync + 'static>);

impl<F, I> From<F> for Callback<I>
where
    F: Fn(I) + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        Self(Arc::new(value))
    }
}

impl<I> Deref for Callback<I> {
    type Target = dyn Fn(I) + Send + Sync + 'static;

    fn deref(&self) -> &Self::Target {
        Arc::deref(&self.0)
    }
}

impl<I> Debug for Callback<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let ptr = Arc::as_ptr(&self.0);
        ptr.fmt(f)
    }
}

impl<I> Clone for Callback<I> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Clone, Debug)]
pub enum Provider<T>
where
    T: Send + Sync + 'static,
{
    Value(T),
    Signal(ReadSignal<T>),
}

impl<T> Provider<T>
where
    T: Send + Sync + 'static,
{
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        match self {
            Self::Value(val) => val.clone(),
            Self::Signal(reader) => reader.get(),
        }
    }

    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        match self {
            Self::Value(val) => val.clone(),
            Self::Signal(reader) => reader.get_untracked(),
        }
    }
}
