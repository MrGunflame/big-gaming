mod button;
mod container;
mod image;
mod input;
mod text;

pub use self::image::{Image, ImageProps};
pub use button::{Button, ButtonProps};
pub use container::{Container, ContainerProps};
pub use input::{Input, InputProps};
pub use text::{Text, TextProp, TextProps};

use std::ops::Deref;

use crate::reactive::Scope;

pub trait Widget {
    type Properties;

    fn render(cx: &Scope, props: Self::Properties) -> Scope;
}

pub trait Component {
    type Properties;

    fn render(cx: &Scope, props: Self::Properties) -> Scope;
}

pub struct Callback<I: 'static>(pub Box<dyn Fn(I) + Send + Sync + 'static>);

impl<I> Default for Callback<I> {
    fn default() -> Self {
        Self(Box::new(|_| {}))
    }
}

impl<F, I> From<F> for Callback<I>
where
    F: Fn(I) + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        Self(Box::new(value))
    }
}

impl<I> Deref for Callback<I> {
    type Target = dyn Fn(I) + Send + Sync + 'static;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
