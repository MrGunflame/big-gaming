// mod button;
// mod checkbox;
// mod container;
// mod image;
mod input;
// mod parse_input;
// mod plot;
// mod selection;
mod text;

// pub mod value_slider;

// pub use self::image::Image;
// pub use button::Button;
// pub use checkbox::Checkbox;
// pub use container::Container;
pub use input::Input;
// pub use parse_input::ParseInput;
// pub use plot::Plot;
// pub use selection::Selection;
pub use text::Text;

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use crate::reactive::Context;

pub trait Widget {
    fn mount<T>(self, parent: &Context<T>);
}
