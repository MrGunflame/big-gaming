use crate::events::ElementEventHandlers;
use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::Widget;

#[derive(Clone, Debug)]
pub struct Text {
    text: String,
}

impl Text {
    pub const fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    pub fn text<T>(mut self, text: T) -> Self
    where
        T: ToString,
    {
        self.text = text.to_string();
        self
    }
}

impl Widget for Text {
    fn build(self, cx: &Scope) -> Scope {
        cx.push(Node {
            element: Element {
                body: ElementBody::Text(crate::render::Text {
                    text: self.text,
                    size: 24.0,
                }),
                style: Style::default(),
            },
            events: ElementEventHandlers::default(),
        })
    }
}
