use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::style::Style;

use super::Widget;

#[derive(Clone, Debug, Default)]
pub struct Container {
    style: Style,
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Container {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        parent.append(Node::new(Primitive {
            style: self.style,
            image: None,
            text: None,
        }))
    }
}
