use crate::reactive::{Node, Scope};
use crate::render::{Element, ElementBody};
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
    fn build(self, cx: &Scope) -> Scope {
        cx.push(Node {
            element: Element {
                body: ElementBody::Container,
                style: self.style,
            },
            events: Default::default(),
        })
    }
}
