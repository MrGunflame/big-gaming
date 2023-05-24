use std::ops::Deref;

use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::Component;

#[derive(Default)]
pub struct ContainerProps {
    pub style: Style,
}

pub struct Container;

impl Component for Container {
    type Properties = ContainerProps;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: props.style,
            },
            events: Default::default(),
        })
    }
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
